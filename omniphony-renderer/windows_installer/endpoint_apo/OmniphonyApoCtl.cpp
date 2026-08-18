#include <windows.h>
#include <audioclient.h>
#include <propkeydef.h>
#include <functiondiscoverykeys_devpkey.h>
#include <mmdeviceapi.h>
#include <propsys.h>
#include <propvarutil.h>
#include <wrl/client.h>

#include <algorithm>
#include <cwctype>
#include <iostream>
#include <string>
#include <vector>

using Microsoft::WRL::ComPtr;

struct DeviceShareMode;

// Windows does not publish IPolicyConfig as a supported SDK interface, but the
// ABI is long-lived and is already used by OmniphonyEndpointCtl for endpoint
// policy operations. Crucially, SetPropertyValue goes through the Windows audio
// service instead of trying to write the protected MMDevices FxProperties ACL.
struct __declspec(uuid("F8679F50-850A-41CF-9C72-430F290290C8")) IPolicyConfig : IUnknown {
    virtual HRESULT STDMETHODCALLTYPE GetMixFormat(PCWSTR, WAVEFORMATEX**) = 0;
    virtual HRESULT STDMETHODCALLTYPE GetDeviceFormat(PCWSTR, INT, WAVEFORMATEX**) = 0;
    virtual HRESULT STDMETHODCALLTYPE ResetDeviceFormat(PCWSTR) = 0;
    virtual HRESULT STDMETHODCALLTYPE SetDeviceFormat(PCWSTR, WAVEFORMATEX*, WAVEFORMATEX*) = 0;
    virtual HRESULT STDMETHODCALLTYPE GetProcessingPeriod(PCWSTR, INT, PINT64, PINT64) = 0;
    virtual HRESULT STDMETHODCALLTYPE SetProcessingPeriod(PCWSTR, PINT64) = 0;
    virtual HRESULT STDMETHODCALLTYPE GetShareMode(PCWSTR, DeviceShareMode*) = 0;
    virtual HRESULT STDMETHODCALLTYPE SetShareMode(PCWSTR, DeviceShareMode*) = 0;
    virtual HRESULT STDMETHODCALLTYPE GetPropertyValue(PCWSTR, const PROPERTYKEY&, PROPVARIANT*) = 0;
    virtual HRESULT STDMETHODCALLTYPE SetPropertyValue(PCWSTR, const PROPERTYKEY&, PROPVARIANT*) = 0;
    virtual HRESULT STDMETHODCALLTYPE SetDefaultEndpoint(PCWSTR, ERole) = 0;
    virtual HRESULT STDMETHODCALLTYPE SetEndpointVisibility(PCWSTR, INT) = 0;
};

class __declspec(uuid("870AF99C-171D-4F9E-AF0D-E63DF40C2BC9")) CPolicyConfigClient;

namespace {

constexpr GUID kOmniphonyApoClsid = {
    0xa9333bfe, 0x39c1, 0x40fd, {0xb4, 0xb0, 0xec, 0xc5, 0x91, 0x41, 0x0b, 0x47}};
constexpr GUID kOmniphonyNativeSurroundApoClsid = {
    0x07d403d9, 0x8a98, 0x43ef, {0x8c, 0x28, 0x86, 0x51, 0x75, 0x6d, 0x83, 0xbe}};
constexpr PROPERTYKEY kEndpointGuid = {
    {0x1da5d803, 0xd492, 0x4edd, {0x8c, 0x23, 0xe0, 0xc0, 0xff, 0xee, 0x7f, 0x0e}}, 4};
constexpr PROPERTYKEY kSfxKey = {
    {0xd04e05a6, 0x594b, 0x4fb6, {0xa8, 0x0d, 0x01, 0xaf, 0x5e, 0xed, 0x7d, 0x1d}}, 5};
constexpr PROPERTYKEY kEfxKey = {
    {0xd04e05a6, 0x594b, 0x4fb6, {0xa8, 0x0d, 0x01, 0xaf, 0x5e, 0xed, 0x7d, 0x1d}}, 7};
constexpr PROPERTYKEY kEfxModesKey = {
    {0xd3993a3f, 0x99c2, 0x4402, {0xb5, 0xec, 0xa9, 0x2a, 0x03, 0x67, 0x66, 0x4b}}, 7};
constexpr PROPERTYKEY kDisableSysFxKey = {
    {0x1da5d803, 0xd492, 0x4edd, {0x8c, 0x23, 0xe0, 0xc0, 0xff, 0xee, 0x7f, 0x0e}}, 5};

constexpr wchar_t kRenderBase[] = L"SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\MMDevices\\Audio\\Render\\";
constexpr wchar_t kSfxValue[] = L"{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},5";
constexpr wchar_t kEfxValue[] = L"{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},7";
constexpr wchar_t kEfxModesValue[] = L"{d3993a3f-99c2-4402-b5ec-a92a0367664b},7";
constexpr wchar_t kDisableSysFxValue[] = L"{1da5d803-d492-4edd-8c23-e0c0ffee7f0e},5";
constexpr wchar_t kDefaultMode[] = L"{C18E2F7E-933D-4965-B7D1-1EEF228D2AF3}";
constexpr wchar_t kStateBase[] = L"SOFTWARE\\Omniphony\\EndpointAPO\\";

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
    StringFromGUID2(guid, text, 64);
    return text;
}

bool IsOmniphonyEfx(const std::wstring& value) {
    return _wcsicmp(value.c_str(), GuidText(kOmniphonyApoClsid).c_str()) == 0 ||
           _wcsicmp(value.c_str(), GuidText(kOmniphonyNativeSurroundApoClsid).c_str()) == 0;
}

std::wstring HResultText(HRESULT hr) {
    wchar_t text[32] = {};
    swprintf_s(text, L"0x%08lX", static_cast<unsigned long>(hr));
    return text;
}

HRESULT GetString(IPropertyStore* store, REFPROPERTYKEY key, std::wstring& value) {
    PROPVARIANT v;
    PropVariantInit(&v);
    const HRESULT hr = store->GetValue(key, &v);
    if (SUCCEEDED(hr) && v.vt == VT_LPWSTR && v.pwszVal) {
        value = v.pwszVal;
    }
    PropVariantClear(&v);
    return hr;
}

HRESULT Enumerate(std::vector<Endpoint>& endpoints) {
    ComPtr<IMMDeviceEnumerator> enumerator;
    HRESULT hr = CoCreateInstance(__uuidof(MMDeviceEnumerator), nullptr, CLSCTX_INPROC_SERVER,
                                  IID_PPV_ARGS(enumerator.ReleaseAndGetAddressOf()));
    if (FAILED(hr)) return hr;

    ComPtr<IMMDeviceCollection> collection;
    hr = enumerator->EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE, collection.ReleaseAndGetAddressOf());
    if (FAILED(hr)) return hr;

    UINT count = 0;
    hr = collection->GetCount(&count);
    if (FAILED(hr)) return hr;

    for (UINT i = 0; i < count; ++i) {
        ComPtr<IMMDevice> device;
        if (FAILED(collection->Item(i, device.ReleaseAndGetAddressOf()))) continue;

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

bool FindEndpoint(const std::vector<std::wstring>& needles, Endpoint& result) {
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
    for (int i = start; i < argc; ++i) {
        if (argv[i] && *argv[i]) values.emplace_back(argv[i]);
    }
    if (values.empty()) {
        values = {L"Dan Clark Noire X", L"FiiO", L"Noire"};
    }
    return values;
}

std::wstring FxPath(const Endpoint& endpoint) {
    return std::wstring(kRenderBase) + endpoint.guid + L"\\FxProperties";
}

bool ReadRegString(const std::wstring& path, const wchar_t* name, std::wstring& value, DWORD* typeOut = nullptr) {
    HKEY key = nullptr;
    if (RegOpenKeyExW(HKEY_LOCAL_MACHINE, path.c_str(), 0, KEY_QUERY_VALUE | KEY_WOW64_64KEY, &key) != ERROR_SUCCESS) return false;
    DWORD type = 0;
    DWORD size = 0;
    LSTATUS status = RegQueryValueExW(key, name, nullptr, &type, nullptr, &size);
    if (status != ERROR_SUCCESS || (type != REG_SZ && type != REG_MULTI_SZ)) {
        RegCloseKey(key);
        return false;
    }
    std::vector<wchar_t> buffer(size / sizeof(wchar_t) + 2, L'\0');
    status = RegQueryValueExW(key, name, nullptr, &type, reinterpret_cast<BYTE*>(buffer.data()), &size);
    RegCloseKey(key);
    if (status != ERROR_SUCCESS) return false;
    value.assign(buffer.data());
    if (typeOut) *typeOut = type;
    return true;
}

bool ReadRegDword(const std::wstring& path, const wchar_t* name, DWORD& value) {
    HKEY key = nullptr;
    if (RegOpenKeyExW(HKEY_LOCAL_MACHINE, path.c_str(), 0, KEY_QUERY_VALUE | KEY_WOW64_64KEY, &key) != ERROR_SUCCESS) return false;
    DWORD type = 0;
    DWORD size = sizeof(value);
    const LSTATUS status = RegQueryValueExW(key, name, nullptr, &type, reinterpret_cast<BYTE*>(&value), &size);
    RegCloseKey(key);
    return status == ERROR_SUCCESS && type == REG_DWORD;
}

LSTATUS OpenStateWritable(const std::wstring& path, HKEY& key) {
    return RegCreateKeyExW(HKEY_LOCAL_MACHINE, path.c_str(), 0, nullptr, 0,
                           KEY_QUERY_VALUE | KEY_SET_VALUE | KEY_WOW64_64KEY,
                           nullptr, &key, nullptr);
}

LSTATUS WriteStateDword(const std::wstring& path, const wchar_t* name, DWORD value) {
    HKEY key = nullptr;
    LSTATUS status = OpenStateWritable(path, key);
    if (status != ERROR_SUCCESS) return status;
    status = RegSetValueExW(key, name, 0, REG_DWORD, reinterpret_cast<const BYTE*>(&value), sizeof(value));
    RegCloseKey(key);
    return status;
}

HRESULT CreatePolicyConfig(ComPtr<IPolicyConfig>& policy) {
    return CoCreateInstance(
        __uuidof(CPolicyConfigClient), nullptr, CLSCTX_ALL, __uuidof(IPolicyConfig),
        reinterpret_cast<void**>(policy.ReleaseAndGetAddressOf()));
}

HRESULT SetPolicyProperty(IPolicyConfig* policy,
                          const Endpoint& endpoint,
                          REFPROPERTYKEY key,
                          PROPVARIANT& value) {
    if (!policy) return E_POINTER;
    return policy->SetPropertyValue(endpoint.id.c_str(), key, &value);
}

HRESULT SetStringProperty(IPolicyConfig* policy,
                          const Endpoint& endpoint,
                          REFPROPERTYKEY key,
                          const std::wstring& value) {
    PROPVARIANT property;
    PropVariantInit(&property);
    HRESULT hr = InitPropVariantFromString(value.c_str(), &property);
    if (SUCCEEDED(hr)) hr = SetPolicyProperty(policy, endpoint, key, property);
    PropVariantClear(&property);
    return hr;
}

HRESULT SetStringVectorProperty(IPolicyConfig* policy,
                                const Endpoint& endpoint,
                                REFPROPERTYKEY key,
                                const std::vector<std::wstring>& values) {
    std::vector<PCWSTR> pointers;
    pointers.reserve(values.size());
    for (const auto& value : values) pointers.push_back(value.c_str());

    PROPVARIANT property;
    PropVariantInit(&property);
    HRESULT hr = InitPropVariantFromStringVector(
        pointers.empty() ? nullptr : pointers.data(),
        static_cast<ULONG>(pointers.size()),
        &property);
    if (SUCCEEDED(hr)) hr = SetPolicyProperty(policy, endpoint, key, property);
    PropVariantClear(&property);
    return hr;
}

HRESULT SetDwordProperty(IPolicyConfig* policy,
                         const Endpoint& endpoint,
                         REFPROPERTYKEY key,
                         DWORD value) {
    PROPVARIANT property;
    PropVariantInit(&property);
    HRESULT hr = InitPropVariantFromUInt32(value, &property);
    if (SUCCEEDED(hr)) hr = SetPolicyProperty(policy, endpoint, key, property);
    PropVariantClear(&property);
    return hr;
}

HRESULT ClearProperty(IPolicyConfig* policy, const Endpoint& endpoint, REFPROPERTYKEY key) {
    PROPVARIANT property;
    PropVariantInit(&property);
    const HRESULT hr = SetPolicyProperty(policy, endpoint, key, property);
    PropVariantClear(&property);
    return hr;
}

int Show(const Endpoint& endpoint) {
    const std::wstring fx = FxPath(endpoint);
    std::wstring efx;
    DWORD disabled = 0;
    const bool hasEfx = ReadRegString(fx, kEfxValue, efx);
    const bool hasDisabled = ReadRegDword(fx, kDisableSysFxValue, disabled);
    std::wcout << L"ENDPOINT\t" << endpoint.name << L"\t" << endpoint.guid << L"\t" << endpoint.id << L"\n";
    std::wcout << L"EFX\t" << (hasEfx ? efx : L"<absent>") << L"\n";
    std::wcout << L"ENHANCEMENTS_DISABLED\t" << (hasDisabled ? disabled : 0) << L"\n";
    return hasEfx && IsOmniphonyEfx(efx) ? 0 : 3;
}

int Attach(const Endpoint& endpoint) {
    const std::wstring fx = FxPath(endpoint);
    const std::wstring ours = GuidText(kOmniphonyApoClsid);
    std::wstring existing;
    if (ReadRegString(fx, kEfxValue, existing) && !IsOmniphonyEfx(existing)) {
        std::wcerr << L"ERROR\tEXISTING_EFX\t" << existing << L"\n";
        return 8;
    }

    std::wstring modes;
    const bool modesExisted = ReadRegString(fx, kEfxModesValue, modes);
    const std::wstring state = std::wstring(kStateBase) + endpoint.guid;
    LSTATUS status = WriteStateDword(state, L"ModesExisted", modesExisted ? 1u : 0u);
    if (status != ERROR_SUCCESS) {
        std::wcerr << L"ERROR\tSTATE_WRITE\t" << status << L"\n";
        return 5;
    }

    ComPtr<IPolicyConfig> policy;
    HRESULT hr = CreatePolicyConfig(policy);
    if (FAILED(hr)) {
        std::wcerr << L"ERROR\tPOLICY_CREATE\t" << HResultText(hr) << L"\n";
        return 5;
    }

    hr = SetStringProperty(policy.Get(), endpoint, kEfxKey, ours);
    if (FAILED(hr)) {
        std::wcerr << L"ERROR\tFX_POLICY_WRITE\t" << HResultText(hr) << L"\n";
        return 5;
    }

    if (!modesExisted) {
        hr = SetStringVectorProperty(policy.Get(), endpoint, kEfxModesKey, {kDefaultMode});
        if (FAILED(hr)) {
            ClearProperty(policy.Get(), endpoint, kEfxKey);
            std::wcerr << L"ERROR\tMODE_POLICY_WRITE\t" << HResultText(hr) << L"\n";
            return 6;
        }
    }

    hr = SetDwordProperty(policy.Get(), endpoint, kDisableSysFxKey, 0);
    if (FAILED(hr)) {
        std::wcerr << L"ERROR\tENABLE_SYSFX_POLICY_WRITE\t" << HResultText(hr) << L"\n";
        return 6;
    }

    std::wcout << L"APO_ATTACHED\t" << endpoint.name << L"\t" << endpoint.guid << L"\t" << endpoint.id << L"\n";
    std::wcout << L"SYSTEM_EFFECTS_ENABLED\t1\n";
    std::wcout << L"RESTART_AUDIO_REQUIRED\t1\n";
    return 0;
}

int AttachNative(const Endpoint& endpoint) {
    const std::wstring fx = FxPath(endpoint);
    std::wstring existing;
    if (ReadRegString(fx, kEfxValue, existing) && !IsOmniphonyEfx(existing)) {
        std::wcerr << L"ERROR\tEXISTING_EFX\t" << existing << L"\n";
        return 8;
    }

    ComPtr<IPolicyConfig> policy;
    HRESULT hr = CreatePolicyConfig(policy);
    if (FAILED(hr)) {
        std::wcerr << L"ERROR\tPOLICY_CREATE\t" << HResultText(hr) << L"\n";
        return 5;
    }

    hr = SetStringProperty(policy.Get(), endpoint, kEfxKey, GuidText(kOmniphonyNativeSurroundApoClsid));
    if (FAILED(hr)) {
        std::wcerr << L"ERROR\tNATIVE_FX_POLICY_WRITE\t" << HResultText(hr) << L"\n";
        return 5;
    }

    hr = SetDwordProperty(policy.Get(), endpoint, kDisableSysFxKey, 0);
    if (FAILED(hr)) {
        std::wcerr << L"ERROR\tENABLE_SYSFX_POLICY_WRITE\t" << HResultText(hr) << L"\n";
        return 6;
    }

    std::wcout << L"APO_NATIVE_ATTACHED\t" << endpoint.name << L"\t" << endpoint.guid << L"\t" << endpoint.id << L"\n";
    std::wcout << L"SYSTEM_EFFECTS_ENABLED\t1\n";
    std::wcout << L"RESTART_AUDIO_REQUIRED\t1\n";
    return 0;
}

int CleanupNativeSfx(const Endpoint& endpoint) {
    const std::wstring fx = FxPath(endpoint);
    std::wstring existing;
    if (!ReadRegString(fx, kSfxValue, existing)) {
        std::wcout << L"LEGACY_NATIVE_SFX\tabsent\n";
        return 0;
    }
    if (_wcsicmp(existing.c_str(), GuidText(kOmniphonyNativeSurroundApoClsid).c_str()) != 0) {
        std::wcout << L"LEGACY_NATIVE_SFX\tforeign\t" << existing << L"\n";
        return 0;
    }

    ComPtr<IPolicyConfig> policy;
    HRESULT hr = CreatePolicyConfig(policy);
    if (FAILED(hr)) {
        std::wcerr << L"ERROR\tPOLICY_CREATE\t" << HResultText(hr) << L"\n";
        return 5;
    }
    hr = ClearProperty(policy.Get(), endpoint, kSfxKey);
    if (FAILED(hr)) {
        std::wcerr << L"ERROR\tLEGACY_SFX_POLICY_CLEAR\t" << HResultText(hr) << L"\n";
        return 6;
    }
    std::wcout << L"LEGACY_NATIVE_SFX\tremoved\n";
    std::wcout << L"RESTART_AUDIO_REQUIRED\t1\n";
    return 0;
}

int Detach(const Endpoint& endpoint) {
    const std::wstring fx = FxPath(endpoint);
    std::wstring existing;

    ComPtr<IPolicyConfig> policy;
    HRESULT hr = CreatePolicyConfig(policy);
    if (FAILED(hr)) {
        std::wcerr << L"ERROR\tPOLICY_CREATE\t" << HResultText(hr) << L"\n";
        return 5;
    }

    if (ReadRegString(fx, kEfxValue, existing) && IsOmniphonyEfx(existing)) {
        hr = ClearProperty(policy.Get(), endpoint, kEfxKey);
        if (FAILED(hr)) {
            std::wcerr << L"ERROR\tFX_POLICY_CLEAR\t" << HResultText(hr) << L"\n";
            return 6;
        }
    }

    const std::wstring state = std::wstring(kStateBase) + endpoint.guid;
    DWORD modesExisted = 1;
    if (ReadRegDword(state, L"ModesExisted", modesExisted) && modesExisted == 0) {
        hr = ClearProperty(policy.Get(), endpoint, kEfxModesKey);
        if (FAILED(hr)) {
            std::wcerr << L"ERROR\tMODE_POLICY_CLEAR\t" << HResultText(hr) << L"\n";
            return 6;
        }
    }
    RegDeleteTreeW(HKEY_LOCAL_MACHINE, state.c_str());

    std::wcout << L"APO_DETACHED\t" << endpoint.name << L"\t" << endpoint.guid << L"\t" << endpoint.id << L"\n";
    std::wcout << L"RESTART_AUDIO_REQUIRED\t1\n";
    return 0;
}

int SetBypass(const Endpoint& endpoint, bool bypass) {
    ComPtr<IPolicyConfig> policy;
    HRESULT hr = CreatePolicyConfig(policy);
    if (FAILED(hr)) {
        std::wcerr << L"ERROR\tPOLICY_CREATE\t" << HResultText(hr) << L"\n";
        return 5;
    }
    hr = SetDwordProperty(policy.Get(), endpoint, kDisableSysFxKey, bypass ? 1u : 0u);
    if (FAILED(hr)) {
        std::wcerr << L"ERROR\tSYSFX_POLICY_WRITE\t" << HResultText(hr) << L"\n";
        return 6;
    }
    std::wcout << (bypass ? L"SYSTEM_EFFECTS_BYPASSED" : L"SYSTEM_EFFECTS_ENABLED")
               << L"\t" << endpoint.name << L"\t" << endpoint.id << L"\n";
    std::wcout << L"RESTART_AUDIO_REQUIRED\t1\n";
    return 0;
}

bool IsIdCommand(const std::wstring& command) {
    return command == L"status-id" || command == L"attach-id" || command == L"attach-native-id" ||
           command == L"cleanup-native-sfx-id" || command == L"detach-id" ||
           command == L"bypass-id" || command == L"enable-effects-id";
}

} // namespace

int wmain(int argc, wchar_t** argv) {
    if (argc < 2) {
        std::wcerr << L"usage: OmniphonyApoCtl <list|status|attach|attach-native|cleanup-native-sfx|detach|bypass|enable-effects|status-id|attach-id|attach-native-id|cleanup-native-sfx-id|detach-id|bypass-id|enable-effects-id> ...\n";
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
            std::wcout << L"ENDPOINT\t" << endpoint.name << L"\t" << endpoint.guid << L"\t" << endpoint.id << L"\n";
        }
        CoUninitialize();
        return 0;
    }

    Endpoint endpoint;
    if (IsIdCommand(command)) {
        if (argc != 3 || !argv[2] || !*argv[2] || !FindEndpointById(argv[2], endpoint)) {
            std::wcerr << L"ERROR\tENDPOINT_ID_NOT_FOUND\n";
            CoUninitialize();
            return 3;
        }
    } else if (!FindEndpoint(Needles(argc, argv, 2), endpoint)) {
        std::wcerr << L"ERROR\tENDPOINT_NOT_FOUND\n";
        CoUninitialize();
        return 3;
    }

    int result = 2;
    if (command == L"status" || command == L"status-id") result = Show(endpoint);
    else if (command == L"attach" || command == L"attach-id") result = Attach(endpoint);
    else if (command == L"attach-native" || command == L"attach-native-id") result = AttachNative(endpoint);
    else if (command == L"cleanup-native-sfx" || command == L"cleanup-native-sfx-id") result = CleanupNativeSfx(endpoint);
    else if (command == L"detach" || command == L"detach-id") result = Detach(endpoint);
    else if (command == L"bypass" || command == L"bypass-id") result = SetBypass(endpoint, true);
    else if (command == L"enable-effects" || command == L"enable-effects-id") result = SetBypass(endpoint, false);
    else std::wcerr << L"ERROR\tUNKNOWN_COMMAND\n";

    CoUninitialize();
    return result;
}
