#include <windows.h>
#include <aclapi.h>
#include <audioclient.h>
#include <propkeydef.h>
#include <functiondiscoverykeys_devpkey.h>
#include <mmdeviceapi.h>
#include <propsys.h>
#include <wrl/client.h>

#include <algorithm>
#include <cwctype>
#include <iostream>
#include <string>
#include <vector>

using Microsoft::WRL::ComPtr;

namespace {

constexpr GUID kOmniphonyApoClsid = {
    0xa9333bfe, 0x39c1, 0x40fd, {0xb4, 0xb0, 0xec, 0xc5, 0x91, 0x41, 0x0b, 0x47}};
constexpr GUID kOmniphonyNativeSurroundApoClsid = {
    0x07d403d9, 0x8a98, 0x43ef, {0x8c, 0x28, 0x86, 0x51, 0x75, 0x6d, 0x83, 0xbe}};
constexpr PROPERTYKEY kEndpointGuid = {
    {0x1da5d803, 0xd492, 0x4edd, {0x8c, 0x23, 0xe0, 0xc0, 0xff, 0xee, 0x7f, 0x0e}}, 4};

constexpr wchar_t kRenderBase[] =
    L"SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\MMDevices\\Audio\\Render\\";
constexpr wchar_t kSfxValue[] = L"{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},5";
constexpr wchar_t kSfxModesValue[] = L"{d3993a3f-99c2-4402-b5ec-a92a0367664b},5";
constexpr wchar_t kEfxValue[] = L"{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},7";
constexpr wchar_t kEfxModesValue[] = L"{d3993a3f-99c2-4402-b5ec-a92a0367664b},7";
constexpr wchar_t kDisableSysFxValue[] = L"{1da5d803-d492-4edd-8c23-e0c0ffee7f0e},5";
constexpr wchar_t kDefaultMode[] = L"{C18E2F7E-933D-4965-B7D1-1EEF228D2AF3}";
constexpr wchar_t kStateBase[] = L"SOFTWARE\\Omniphony\\EndpointAPO\\";
constexpr wchar_t kSfxStateBase[] = L"SOFTWARE\\Omniphony\\NativeSFX\\";

struct Endpoint {
    std::wstring id;
    std::wstring name;
    std::wstring guid;
};

std::wstring Lower(std::wstring value) {
    std::transform(value.begin(), value.end(), value.begin(), [](wchar_t c) {
        return static_cast<wchar_t>(std::towlower(c));
    });
    return value;
}

bool ContainsInsensitive(const std::wstring& haystack, const std::wstring& needle) {
    return !needle.empty() && Lower(haystack).find(Lower(needle)) != std::wstring::npos;
}

std::wstring GuidText(REFGUID guid) {
    wchar_t text[64] = {};
    StringFromGUID2(guid, text, static_cast<int>(sizeof(text) / sizeof(text[0])));
    return text;
}

bool IsOmniphonyFx(const std::wstring& value) {
    return _wcsicmp(value.c_str(), GuidText(kOmniphonyApoClsid).c_str()) == 0 ||
           _wcsicmp(value.c_str(), GuidText(kOmniphonyNativeSurroundApoClsid).c_str()) == 0;
}

HRESULT GetString(IPropertyStore* store, REFPROPERTYKEY key, std::wstring& value) {
    PROPVARIANT property;
    PropVariantInit(&property);
    const HRESULT hr = store->GetValue(key, &property);
    if (SUCCEEDED(hr) && property.vt == VT_LPWSTR && property.pwszVal) {
        value = property.pwszVal;
    }
    PropVariantClear(&property);
    return hr;
}

HRESULT Enumerate(std::vector<Endpoint>& endpoints) {
    ComPtr<IMMDeviceEnumerator> enumerator;
    HRESULT hr = CoCreateInstance(
        __uuidof(MMDeviceEnumerator), nullptr, CLSCTX_INPROC_SERVER,
        IID_PPV_ARGS(enumerator.ReleaseAndGetAddressOf()));
    if (FAILED(hr)) return hr;

    ComPtr<IMMDeviceCollection> collection;
    hr = enumerator->EnumAudioEndpoints(
        eRender, DEVICE_STATE_ACTIVE, collection.ReleaseAndGetAddressOf());
    if (FAILED(hr)) return hr;

    UINT count = 0;
    hr = collection->GetCount(&count);
    if (FAILED(hr)) return hr;

    for (UINT index = 0; index < count; ++index) {
        ComPtr<IMMDevice> device;
        if (FAILED(collection->Item(index, device.ReleaseAndGetAddressOf()))) continue;

        LPWSTR rawId = nullptr;
        if (FAILED(device->GetId(&rawId))) continue;
        Endpoint endpoint;
        endpoint.id = rawId ? rawId : L"";
        CoTaskMemFree(rawId);

        ComPtr<IPropertyStore> store;
        if (FAILED(device->OpenPropertyStore(STGM_READ, store.ReleaseAndGetAddressOf()))) continue;
        if (FAILED(GetString(store.Get(), PKEY_Device_FriendlyName, endpoint.name))) continue;
        if (FAILED(GetString(store.Get(), kEndpointGuid, endpoint.guid))) continue;
        endpoints.push_back(std::move(endpoint));
    }
    return S_OK;
}

bool FindEndpointByName(const std::vector<std::wstring>& needles, Endpoint& result) {
    std::vector<Endpoint> endpoints;
    if (FAILED(Enumerate(endpoints))) return false;
    for (const auto& needle : needles) {
        for (const auto& endpoint : endpoints) {
            if (ContainsInsensitive(endpoint.name, needle)) {
                result = endpoint;
                return true;
            }
        }
    }
    return false;
}

bool FindEndpointById(const std::wstring& id, Endpoint& result) {
    std::vector<Endpoint> endpoints;
    if (FAILED(Enumerate(endpoints))) return false;
    for (const auto& endpoint : endpoints) {
        if (_wcsicmp(endpoint.id.c_str(), id.c_str()) == 0) {
            result = endpoint;
            return true;
        }
    }
    return false;
}

std::vector<std::wstring> Needles(int argc, wchar_t** argv, int start) {
    std::vector<std::wstring> values;
    for (int index = start; index < argc; ++index) {
        if (argv[index] && *argv[index]) values.emplace_back(argv[index]);
    }
    if (values.empty()) values = {L"Dan Clark Noire X", L"FiiO", L"Noire"};
    return values;
}

std::wstring EndpointPath(const Endpoint& endpoint) {
    return std::wstring(kRenderBase) + endpoint.guid;
}

std::wstring FxPath(const Endpoint& endpoint) {
    return EndpointPath(endpoint) + L"\\FxProperties";
}

bool ReadRegString(const std::wstring& path, const wchar_t* name, std::wstring& value,
                   DWORD* typeOut = nullptr) {
    HKEY key = nullptr;
    if (RegOpenKeyExW(HKEY_LOCAL_MACHINE, path.c_str(), 0,
                      KEY_QUERY_VALUE | KEY_WOW64_64KEY, &key) != ERROR_SUCCESS) {
        return false;
    }
    DWORD type = 0;
    DWORD size = 0;
    LSTATUS status = RegQueryValueExW(key, name, nullptr, &type, nullptr, &size);
    if (status != ERROR_SUCCESS || (type != REG_SZ && type != REG_MULTI_SZ)) {
        RegCloseKey(key);
        return false;
    }
    std::vector<wchar_t> buffer(size / sizeof(wchar_t) + 2, L'\0');
    status = RegQueryValueExW(
        key, name, nullptr, &type, reinterpret_cast<BYTE*>(buffer.data()), &size);
    RegCloseKey(key);
    if (status != ERROR_SUCCESS) return false;
    value.assign(buffer.data());
    if (typeOut) *typeOut = type;
    return true;
}

bool ReadRegDword(const std::wstring& path, const wchar_t* name, DWORD& value) {
    HKEY key = nullptr;
    if (RegOpenKeyExW(HKEY_LOCAL_MACHINE, path.c_str(), 0,
                      KEY_QUERY_VALUE | KEY_WOW64_64KEY, &key) != ERROR_SUCCESS) {
        return false;
    }
    DWORD type = 0;
    DWORD size = sizeof(value);
    const LSTATUS status = RegQueryValueExW(
        key, name, nullptr, &type, reinterpret_cast<BYTE*>(&value), &size);
    RegCloseKey(key);
    return status == ERROR_SUCCESS && type == REG_DWORD;
}

bool ValueExists(const std::wstring& path, const wchar_t* name) {
    HKEY key = nullptr;
    if (RegOpenKeyExW(HKEY_LOCAL_MACHINE, path.c_str(), 0,
                      KEY_QUERY_VALUE | KEY_WOW64_64KEY, &key) != ERROR_SUCCESS) {
        return false;
    }
    const LSTATUS status = RegQueryValueExW(key, name, nullptr, nullptr, nullptr, nullptr);
    RegCloseKey(key);
    return status == ERROR_SUCCESS;
}

bool EnablePrivilege(const wchar_t* privilege, bool enable) {
    HANDLE token = nullptr;
    if (!OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY, &token)) {
        return false;
    }
    LUID luid = {};
    if (!LookupPrivilegeValueW(nullptr, privilege, &luid)) {
        CloseHandle(token);
        return false;
    }
    TOKEN_PRIVILEGES privileges = {};
    privileges.PrivilegeCount = 1;
    privileges.Privileges[0].Luid = luid;
    privileges.Privileges[0].Attributes = enable ? SE_PRIVILEGE_ENABLED : 0;
    SetLastError(ERROR_SUCCESS);
    const BOOL adjusted = AdjustTokenPrivileges(
        token, FALSE, &privileges, sizeof(privileges), nullptr, nullptr);
    const DWORD error = GetLastError();
    CloseHandle(token);
    return adjusted != FALSE && error == ERROR_SUCCESS;
}

PSID AllocateAdministratorsSid() {
    PSID sid = nullptr;
    SID_IDENTIFIER_AUTHORITY authority = SECURITY_NT_AUTHORITY;
    if (!AllocateAndInitializeSid(
            &authority, 2, SECURITY_BUILTIN_DOMAIN_RID, DOMAIN_ALIAS_RID_ADMINS,
            0, 0, 0, 0, 0, 0, &sid)) {
        return nullptr;
    }
    return sid;
}

LSTATUS TakeOwnership(const std::wstring& path) {
    if (!EnablePrivilege(SE_TAKE_OWNERSHIP_NAME, true)) return ERROR_PRIVILEGE_NOT_HELD;
    HKEY key = nullptr;
    LSTATUS status = RegOpenKeyExW(
        HKEY_LOCAL_MACHINE, path.c_str(), 0, WRITE_OWNER | KEY_WOW64_64KEY, &key);
    if (status != ERROR_SUCCESS) {
        EnablePrivilege(SE_TAKE_OWNERSHIP_NAME, false);
        return status;
    }
    PSID administrators = AllocateAdministratorsSid();
    if (!administrators) {
        RegCloseKey(key);
        EnablePrivilege(SE_TAKE_OWNERSHIP_NAME, false);
        return GetLastError();
    }
    SECURITY_DESCRIPTOR descriptor = {};
    if (!InitializeSecurityDescriptor(&descriptor, SECURITY_DESCRIPTOR_REVISION) ||
        !SetSecurityDescriptorOwner(&descriptor, administrators, FALSE)) {
        status = GetLastError();
    } else {
        status = RegSetKeySecurity(key, OWNER_SECURITY_INFORMATION, &descriptor);
    }
    FreeSid(administrators);
    RegCloseKey(key);
    EnablePrivilege(SE_TAKE_OWNERSHIP_NAME, false);
    return status;
}

LSTATUS GrantAdministrators(const std::wstring& path) {
    HKEY key = nullptr;
    LSTATUS status = RegOpenKeyExW(
        HKEY_LOCAL_MACHINE, path.c_str(), 0,
        READ_CONTROL | WRITE_DAC | KEY_WOW64_64KEY, &key);
    if (status != ERROR_SUCCESS) return status;

    DWORD descriptorSize = 0;
    status = RegGetKeySecurity(key, DACL_SECURITY_INFORMATION, nullptr, &descriptorSize);
    if (status != ERROR_INSUFFICIENT_BUFFER || descriptorSize == 0) {
        RegCloseKey(key);
        return status;
    }
    std::vector<BYTE> descriptorBytes(descriptorSize, 0);
    auto* descriptor = reinterpret_cast<PSECURITY_DESCRIPTOR>(descriptorBytes.data());
    status = RegGetKeySecurity(key, DACL_SECURITY_INFORMATION, descriptor, &descriptorSize);
    if (status != ERROR_SUCCESS) {
        RegCloseKey(key);
        return status;
    }

    BOOL daclPresent = FALSE;
    BOOL daclDefaulted = FALSE;
    PACL oldAcl = nullptr;
    if (!GetSecurityDescriptorDacl(descriptor, &daclPresent, &oldAcl, &daclDefaulted)) {
        status = GetLastError();
        RegCloseKey(key);
        return status;
    }

    PSID administrators = AllocateAdministratorsSid();
    if (!administrators) {
        status = GetLastError();
        RegCloseKey(key);
        return status;
    }
    EXPLICIT_ACCESSW access = {};
    access.grfAccessPermissions = KEY_ALL_ACCESS;
    access.grfAccessMode = GRANT_ACCESS;
    access.grfInheritance = SUB_CONTAINERS_AND_OBJECTS_INHERIT;
    access.Trustee.TrusteeForm = TRUSTEE_IS_SID;
    access.Trustee.TrusteeType = TRUSTEE_IS_GROUP;
    access.Trustee.ptstrName = static_cast<LPWSTR>(administrators);

    PACL newAcl = nullptr;
    status = SetEntriesInAclW(1, &access, daclPresent ? oldAcl : nullptr, &newAcl);
    if (status == ERROR_SUCCESS) {
        SECURITY_DESCRIPTOR writableDescriptor = {};
        if (!InitializeSecurityDescriptor(&writableDescriptor, SECURITY_DESCRIPTOR_REVISION) ||
            !SetSecurityDescriptorDacl(&writableDescriptor, TRUE, newAcl, FALSE)) {
            status = GetLastError();
        } else {
            status = RegSetKeySecurity(key, DACL_SECURITY_INFORMATION, &writableDescriptor);
        }
    }
    if (newAcl) LocalFree(newAcl);
    FreeSid(administrators);
    RegCloseKey(key);
    return status;
}

LSTATUS MakeWritable(const std::wstring& path) {
    HKEY test = nullptr;
    LSTATUS status = RegOpenKeyExW(
        HKEY_LOCAL_MACHINE, path.c_str(), 0,
        KEY_QUERY_VALUE | KEY_SET_VALUE | KEY_WOW64_64KEY, &test);
    if (status == ERROR_SUCCESS) {
        RegCloseKey(test);
        return ERROR_SUCCESS;
    }
    if (status != ERROR_ACCESS_DENIED) return status;
    status = TakeOwnership(path);
    if (status != ERROR_SUCCESS) return status;
    status = GrantAdministrators(path);
    if (status != ERROR_SUCCESS) return status;
    status = RegOpenKeyExW(
        HKEY_LOCAL_MACHINE, path.c_str(), 0,
        KEY_QUERY_VALUE | KEY_SET_VALUE | KEY_WOW64_64KEY, &test);
    if (status == ERROR_SUCCESS) RegCloseKey(test);
    return status;
}

LSTATUS OpenWritableFx(const Endpoint& endpoint, HKEY& key) {
    key = nullptr;
    const std::wstring endpointPath = EndpointPath(endpoint);
    const std::wstring fxPath = FxPath(endpoint);
    LSTATUS status = RegOpenKeyExW(
        HKEY_LOCAL_MACHINE, fxPath.c_str(), 0,
        KEY_QUERY_VALUE | KEY_SET_VALUE | KEY_WOW64_64KEY, &key);
    if (status == ERROR_SUCCESS) return status;

    if (status == ERROR_FILE_NOT_FOUND || status == ERROR_PATH_NOT_FOUND) {
        status = MakeWritable(endpointPath);
        if (status != ERROR_SUCCESS) return status;
        return RegCreateKeyExW(
            HKEY_LOCAL_MACHINE, fxPath.c_str(), 0, nullptr, 0,
            KEY_QUERY_VALUE | KEY_SET_VALUE | KEY_WOW64_64KEY,
            nullptr, &key, nullptr);
    }
    if (status != ERROR_ACCESS_DENIED) return status;
    status = MakeWritable(endpointPath);
    if (status != ERROR_SUCCESS) return status;
    status = MakeWritable(fxPath);
    if (status != ERROR_SUCCESS) return status;
    status = RegOpenKeyExW(
        HKEY_LOCAL_MACHINE, fxPath.c_str(), 0,
        KEY_QUERY_VALUE | KEY_SET_VALUE | KEY_WOW64_64KEY, &key);
    if (status == ERROR_SUCCESS) {
        std::wcout << L"FX_ACL_REPAIRED\t" << endpoint.name << L"\t" << endpoint.guid << L"\n";
    }
    return status;
}

LSTATUS WriteString(HKEY key, const wchar_t* name, const std::wstring& value) {
    return RegSetValueExW(
        key, name, 0, REG_SZ, reinterpret_cast<const BYTE*>(value.c_str()),
        static_cast<DWORD>((value.size() + 1) * sizeof(wchar_t)));
}

LSTATUS WriteDefaultMode(HKEY key, const wchar_t* name) {
    const size_t length = wcslen(kDefaultMode);
    std::vector<wchar_t> data(length + 2, L'\0');
    std::copy(kDefaultMode, kDefaultMode + length, data.begin());
    return RegSetValueExW(
        key, name, 0, REG_MULTI_SZ, reinterpret_cast<const BYTE*>(data.data()),
        static_cast<DWORD>(data.size() * sizeof(wchar_t)));
}

LSTATUS WriteDword(HKEY key, const wchar_t* name, DWORD value) {
    return RegSetValueExW(
        key, name, 0, REG_DWORD, reinterpret_cast<const BYTE*>(&value), sizeof(value));
}

LSTATUS DeleteValue(HKEY key, const wchar_t* name) {
    const LSTATUS status = RegDeleteValueW(key, name);
    return status == ERROR_FILE_NOT_FOUND ? ERROR_SUCCESS : status;
}

LSTATUS WriteStateDword(const std::wstring& path, const wchar_t* name, DWORD value) {
    HKEY key = nullptr;
    LSTATUS status = RegCreateKeyExW(
        HKEY_LOCAL_MACHINE, path.c_str(), 0, nullptr, 0,
        KEY_QUERY_VALUE | KEY_SET_VALUE | KEY_WOW64_64KEY,
        nullptr, &key, nullptr);
    if (status != ERROR_SUCCESS) return status;
    status = WriteDword(key, name, value);
    RegCloseKey(key);
    return status;
}

void RemoveState(const std::wstring& path) {
    RegDeleteTreeW(HKEY_LOCAL_MACHINE, path.c_str());
}

bool VerifyString(const std::wstring& path, const wchar_t* name, const std::wstring& expected) {
    std::wstring actual;
    return ReadRegString(path, name, actual) && _wcsicmp(actual.c_str(), expected.c_str()) == 0;
}

int Show(const Endpoint& endpoint) {
    const std::wstring fx = FxPath(endpoint);
    std::wstring efx;
    std::wstring sfx;
    DWORD disabled = 0;
    const bool hasEfx = ReadRegString(fx, kEfxValue, efx);
    const bool hasSfx = ReadRegString(fx, kSfxValue, sfx);
    const bool hasDisabled = ReadRegDword(fx, kDisableSysFxValue, disabled);
    std::wcout << L"ENDPOINT\t" << endpoint.name << L"\t" << endpoint.guid << L"\t" << endpoint.id << L"\n";
    std::wcout << L"EFX\t" << (hasEfx ? efx : L"<absent>") << L"\n";
    std::wcout << L"SFX\t" << (hasSfx ? sfx : L"<absent>") << L"\n";
    std::wcout << L"ENHANCEMENTS_DISABLED\t" << (hasDisabled ? disabled : 0) << L"\n";
    return (hasEfx && IsOmniphonyFx(efx)) ||
                   (hasSfx && _wcsicmp(sfx.c_str(), GuidText(kOmniphonyNativeSurroundApoClsid).c_str()) == 0)
               ? 0
               : 3;
}

int AttachEfx(const Endpoint& endpoint, const GUID& clsid, const wchar_t* label) {
    const std::wstring fx = FxPath(endpoint);
    const std::wstring ours = GuidText(clsid);
    std::wstring existing;
    if (ReadRegString(fx, kEfxValue, existing) && !IsOmniphonyFx(existing)) {
        std::wcerr << L"ERROR\tEXISTING_EFX\t" << existing << L"\n";
        return 8;
    }

    const bool modesExisted = ValueExists(fx, kEfxModesValue);
    const std::wstring state = std::wstring(kStateBase) + endpoint.guid;
    LSTATUS status = WriteStateDword(state, L"ModesExisted", modesExisted ? 1u : 0u);
    if (status != ERROR_SUCCESS) {
        std::wcerr << L"ERROR\tSTATE_WRITE\t" << status << L"\n";
        return 5;
    }

    HKEY key = nullptr;
    status = OpenWritableFx(endpoint, key);
    if (status != ERROR_SUCCESS) {
        std::wcerr << L"ERROR\tFX_ACL_WRITE\t" << status << L"\n";
        return 5;
    }
    status = WriteString(key, kEfxValue, ours);
    if (status == ERROR_SUCCESS && !modesExisted) status = WriteDefaultMode(key, kEfxModesValue);
    if (status == ERROR_SUCCESS) status = DeleteValue(key, kDisableSysFxValue);
    RegCloseKey(key);
    if (status != ERROR_SUCCESS) {
        std::wcerr << L"ERROR\tFX_WRITE\t" << status << L"\n";
        return 6;
    }
    if (!VerifyString(fx, kEfxValue, ours)) {
        std::wcerr << L"ERROR\tFX_VERIFY\n";
        return 6;
    }
    DWORD disabled = 0;
    if (ReadRegDword(fx, kDisableSysFxValue, disabled) && disabled != 0) {
        std::wcerr << L"ERROR\tSYSFX_ENABLE_VERIFY\t" << disabled << L"\n";
        return 6;
    }

    std::wcout << label << L"\t" << endpoint.name << L"\t" << endpoint.guid << L"\t" << endpoint.id << L"\n";
    std::wcout << L"FX_REGISTRY_VERIFY_OK\tEFX\t" << ours << L"\n";
    std::wcout << L"SYSTEM_EFFECTS_ENABLED\t1\n";
    std::wcout << L"RESTART_AUDIO_REQUIRED\t1\n";
    return 0;
}

int Attach(const Endpoint& endpoint) {
    return AttachEfx(endpoint, kOmniphonyApoClsid, L"APO_ATTACHED");
}

int AttachNative(const Endpoint& endpoint) {
    return AttachEfx(endpoint, kOmniphonyNativeSurroundApoClsid, L"APO_NATIVE_ATTACHED");
}

int AttachNativeSfx(const Endpoint& endpoint) {
    const std::wstring fx = FxPath(endpoint);
    const std::wstring ours = GuidText(kOmniphonyNativeSurroundApoClsid);
    std::wstring existing;
    if (ReadRegString(fx, kSfxValue, existing) && _wcsicmp(existing.c_str(), ours.c_str()) != 0) {
        std::wcerr << L"ERROR\tEXISTING_SFX\t" << existing << L"\n";
        return 8;
    }

    const bool modesExisted = ValueExists(fx, kSfxModesValue);
    const std::wstring state = std::wstring(kSfxStateBase) + endpoint.guid;
    LSTATUS status = WriteStateDword(state, L"ModesExisted", modesExisted ? 1u : 0u);
    if (status != ERROR_SUCCESS) {
        std::wcerr << L"ERROR\tSFX_STATE_WRITE\t" << status << L"\n";
        return 5;
    }

    HKEY key = nullptr;
    status = OpenWritableFx(endpoint, key);
    if (status != ERROR_SUCCESS) {
        std::wcerr << L"ERROR\tSFX_ACL_WRITE\t" << status << L"\n";
        return 5;
    }
    status = WriteString(key, kSfxValue, ours);
    if (status == ERROR_SUCCESS && !modesExisted) status = WriteDefaultMode(key, kSfxModesValue);
    if (status == ERROR_SUCCESS) status = DeleteValue(key, kDisableSysFxValue);
    RegCloseKey(key);
    if (status != ERROR_SUCCESS) {
        std::wcerr << L"ERROR\tNATIVE_SFX_WRITE\t" << status << L"\n";
        return 6;
    }
    if (!VerifyString(fx, kSfxValue, ours)) {
        std::wcerr << L"ERROR\tNATIVE_SFX_VERIFY\n";
        return 6;
    }
    DWORD disabled = 0;
    if (ReadRegDword(fx, kDisableSysFxValue, disabled) && disabled != 0) {
        std::wcerr << L"ERROR\tSYSFX_ENABLE_VERIFY\t" << disabled << L"\n";
        return 6;
    }

    std::wcout << L"APO_NATIVE_SFX_ATTACHED\t" << endpoint.name << L"\t" << endpoint.guid << L"\t" << endpoint.id << L"\n";
    std::wcout << L"FX_REGISTRY_VERIFY_OK\tSFX\t" << ours << L"\n";
    std::wcout << L"SYSTEM_EFFECTS_ENABLED\t1\n";
    std::wcout << L"RESTART_AUDIO_REQUIRED\t1\n";
    return 0;
}

int CleanupNativeSfx(const Endpoint& endpoint) {
    const std::wstring fx = FxPath(endpoint);
    const std::wstring ours = GuidText(kOmniphonyNativeSurroundApoClsid);
    std::wstring existing;
    const bool hasSfx = ReadRegString(fx, kSfxValue, existing);
    if (hasSfx && _wcsicmp(existing.c_str(), ours.c_str()) != 0) {
        std::wcout << L"LEGACY_NATIVE_SFX\tforeign\t" << existing << L"\n";
        return 0;
    }

    const std::wstring state = std::wstring(kSfxStateBase) + endpoint.guid;
    DWORD modesExisted = 1;
    const bool removeModes = ReadRegDword(state, L"ModesExisted", modesExisted) && modesExisted == 0;
    if (!hasSfx && !removeModes) {
        RemoveState(state);
        std::wcout << L"LEGACY_NATIVE_SFX\tabsent\n";
        return 0;
    }

    HKEY key = nullptr;
    LSTATUS status = OpenWritableFx(endpoint, key);
    if (status != ERROR_SUCCESS) {
        std::wcerr << L"ERROR\tSFX_ACL_WRITE\t" << status << L"\n";
        return 5;
    }
    if (hasSfx) status = DeleteValue(key, kSfxValue);
    if (status == ERROR_SUCCESS && removeModes) status = DeleteValue(key, kSfxModesValue);
    RegCloseKey(key);
    if (status != ERROR_SUCCESS) {
        std::wcerr << L"ERROR\tLEGACY_SFX_DELETE\t" << status << L"\n";
        return 6;
    }
    if (hasSfx) {
        std::wstring verify;
        if (ReadRegString(fx, kSfxValue, verify) && _wcsicmp(verify.c_str(), ours.c_str()) == 0) {
            std::wcerr << L"ERROR\tLEGACY_SFX_VERIFY\n";
            return 6;
        }
    }
    RemoveState(state);
    std::wcout << L"LEGACY_NATIVE_SFX\t" << (hasSfx ? L"removed" : L"absent") << L"\n";
    if (hasSfx) {
        std::wcout << L"FX_REGISTRY_VERIFY_OK\tSFX\t<absent>\n";
        std::wcout << L"RESTART_AUDIO_REQUIRED\t1\n";
    }
    return 0;
}

int Detach(const Endpoint& endpoint) {
    const std::wstring fx = FxPath(endpoint);
    std::wstring existing;
    const bool oursAttached = ReadRegString(fx, kEfxValue, existing) && IsOmniphonyFx(existing);
    const std::wstring state = std::wstring(kStateBase) + endpoint.guid;
    DWORD modesExisted = 1;
    const bool removeModes = ReadRegDword(state, L"ModesExisted", modesExisted) && modesExisted == 0;

    if (!oursAttached && !removeModes) {
        RemoveState(state);
        std::wcout << L"APO_DETACHED\t" << endpoint.name << L"\t" << endpoint.guid << L"\t" << endpoint.id << L"\n";
        std::wcout << L"RESTART_AUDIO_REQUIRED\t1\n";
        return 0;
    }

    HKEY key = nullptr;
    LSTATUS status = OpenWritableFx(endpoint, key);
    if (status != ERROR_SUCCESS) {
        std::wcerr << L"ERROR\tFX_ACL_WRITE\t" << status << L"\n";
        return 5;
    }
    if (oursAttached) status = DeleteValue(key, kEfxValue);
    if (status == ERROR_SUCCESS && removeModes) status = DeleteValue(key, kEfxModesValue);
    RegCloseKey(key);
    if (status != ERROR_SUCCESS) {
        std::wcerr << L"ERROR\tFX_DELETE\t" << status << L"\n";
        return 6;
    }
    if (oursAttached) {
        std::wstring verify;
        if (ReadRegString(fx, kEfxValue, verify) && IsOmniphonyFx(verify)) {
            std::wcerr << L"ERROR\tFX_DETACH_VERIFY\n";
            return 6;
        }
    }
    RemoveState(state);
    std::wcout << L"APO_DETACHED\t" << endpoint.name << L"\t" << endpoint.guid << L"\t" << endpoint.id << L"\n";
    std::wcout << L"FX_REGISTRY_VERIFY_OK\tEFX\t<absent>\n";
    std::wcout << L"RESTART_AUDIO_REQUIRED\t1\n";
    return 0;
}

int SetBypass(const Endpoint& endpoint, bool bypass) {
    const std::wstring fx = FxPath(endpoint);
    HKEY key = nullptr;
    LSTATUS status = OpenWritableFx(endpoint, key);
    if (status != ERROR_SUCCESS) {
        std::wcerr << L"ERROR\tFX_ACL_WRITE\t" << status << L"\n";
        return 5;
    }
    status = bypass ? WriteDword(key, kDisableSysFxValue, 1u)
                    : DeleteValue(key, kDisableSysFxValue);
    RegCloseKey(key);
    if (status != ERROR_SUCCESS) {
        std::wcerr << L"ERROR\tSYSFX_WRITE\t" << status << L"\n";
        return 6;
    }

    DWORD disabled = 0;
    const bool hasDisabled = ReadRegDword(fx, kDisableSysFxValue, disabled);
    if ((bypass && (!hasDisabled || disabled != 1)) ||
        (!bypass && hasDisabled && disabled != 0)) {
        std::wcerr << L"ERROR\tSYSFX_VERIFY\n";
        return 6;
    }

    std::wcout << (bypass ? L"SYSTEM_EFFECTS_BYPASSED" : L"SYSTEM_EFFECTS_ENABLED")
               << L"\t" << endpoint.name << L"\t" << endpoint.id << L"\n";
    std::wcout << L"FX_REGISTRY_VERIFY_OK\tSYSFX\t" << (bypass ? L"1" : L"0") << L"\n";
    std::wcout << L"RESTART_AUDIO_REQUIRED\t1\n";
    return 0;
}

bool IsIdCommand(const std::wstring& command) {
    return command == L"status-id" || command == L"attach-id" || command == L"attach-native-id" ||
           command == L"attach-native-sfx-id" || command == L"cleanup-native-sfx-id" ||
           command == L"detach-id" || command == L"bypass-id" || command == L"enable-effects-id";
}

int Dispatch(const std::wstring& command, const Endpoint& endpoint) {
    if (command == L"status" || command == L"status-id") return Show(endpoint);
    if (command == L"attach" || command == L"attach-id") return Attach(endpoint);
    if (command == L"attach-native" || command == L"attach-native-id") return AttachNative(endpoint);
    if (command == L"attach-native-sfx" || command == L"attach-native-sfx-id") return AttachNativeSfx(endpoint);
    if (command == L"cleanup-native-sfx" || command == L"cleanup-native-sfx-id") return CleanupNativeSfx(endpoint);
    if (command == L"detach" || command == L"detach-id") return Detach(endpoint);
    if (command == L"bypass" || command == L"bypass-id") return SetBypass(endpoint, true);
    if (command == L"enable-effects" || command == L"enable-effects-id") return SetBypass(endpoint, false);
    return 2;
}

} // namespace

int wmain(int argc, wchar_t** argv) {
    if (argc < 2) {
        std::wcerr << L"usage: OmniphonyApoCtl <list|status|attach|attach-native|attach-native-sfx|cleanup-native-sfx|detach|bypass|enable-effects|status-id|attach-id|attach-native-id|attach-native-sfx-id|cleanup-native-sfx-id|detach-id|bypass-id|enable-effects-id> ...\n";
        return 2;
    }

    const HRESULT init = CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);
    if (FAILED(init)) return 4;

    const std::wstring command = argv[1];
    if (command == L"list") {
        std::vector<Endpoint> endpoints;
        const HRESULT hr = Enumerate(endpoints);
        if (FAILED(hr)) {
            CoUninitialize();
            return 4;
        }
        for (const auto& endpoint : endpoints) {
            std::wcout << L"ENDPOINT\t" << endpoint.name << L"\t" << endpoint.guid
                       << L"\t" << endpoint.id << L"\n";
        }
        CoUninitialize();
        return 0;
    }

    Endpoint endpoint;
    bool found = false;
    if (IsIdCommand(command)) {
        if (argc != 3 || !argv[2] || !*argv[2]) {
            std::wcerr << L"ERROR\tID_REQUIRED\n";
            CoUninitialize();
            return 2;
        }
        found = FindEndpointById(argv[2], endpoint);
    } else {
        found = FindEndpointByName(Needles(argc, argv, 2), endpoint);
    }

    if (!found) {
        std::wcerr << L"ERROR\tENDPOINT_NOT_FOUND\n";
        CoUninitialize();
        return 3;
    }

    const int result = Dispatch(command, endpoint);
    if (result == 2) std::wcerr << L"ERROR\tUNKNOWN_COMMAND\n";
    CoUninitialize();
    return result;
}
