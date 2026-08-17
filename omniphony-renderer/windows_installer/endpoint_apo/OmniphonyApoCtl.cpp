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

namespace {

constexpr GUID kOmniphonyApoClsid = {
    0xa9333bfe, 0x39c1, 0x40fd, {0xb4, 0xb0, 0xec, 0xc5, 0x91, 0x41, 0x0b, 0x47}};
constexpr PROPERTYKEY kEndpointGuid = {
    {0x1da5d803, 0xd492, 0x4edd, {0x8c, 0x23, 0xe0, 0xc0, 0xff, 0xee, 0x7f, 0x0e}}, 4};

constexpr wchar_t kRenderBase[] = L"SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\MMDevices\\Audio\\Render\\";
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

LSTATUS OpenWritable(const std::wstring& path, HKEY& key) {
    return RegCreateKeyExW(HKEY_LOCAL_MACHINE, path.c_str(), 0, nullptr, 0,
                           KEY_QUERY_VALUE | KEY_SET_VALUE | KEY_WOW64_64KEY,
                           nullptr, &key, nullptr);
}

LSTATUS WriteRegString(const std::wstring& path, const wchar_t* name, const std::wstring& value) {
    HKEY key = nullptr;
    LSTATUS status = OpenWritable(path, key);
    if (status != ERROR_SUCCESS) return status;
    status = RegSetValueExW(key, name, 0, REG_SZ, reinterpret_cast<const BYTE*>(value.c_str()),
                            static_cast<DWORD>((value.size() + 1) * sizeof(wchar_t)));
    RegCloseKey(key);
    return status;
}

LSTATUS WriteRegDword(const std::wstring& path, const wchar_t* name, DWORD value) {
    HKEY key = nullptr;
    LSTATUS status = OpenWritable(path, key);
    if (status != ERROR_SUCCESS) return status;
    status = RegSetValueExW(key, name, 0, REG_DWORD, reinterpret_cast<const BYTE*>(&value), sizeof(value));
    RegCloseKey(key);
    return status;
}

LSTATUS WriteDefaultMode(const std::wstring& path) {
    HKEY key = nullptr;
    LSTATUS status = OpenWritable(path, key);
    if (status != ERROR_SUCCESS) return status;
    const size_t chars = wcslen(kDefaultMode) + 2;
    std::vector<wchar_t> value(chars, L'\0');
    std::copy(kDefaultMode, kDefaultMode + wcslen(kDefaultMode), value.begin());
    status = RegSetValueExW(key, kEfxModesValue, 0, REG_MULTI_SZ,
                            reinterpret_cast<const BYTE*>(value.data()),
                            static_cast<DWORD>(value.size() * sizeof(wchar_t)));
    RegCloseKey(key);
    return status;
}

LSTATUS DeleteValue(const std::wstring& path, const wchar_t* name) {
    HKEY key = nullptr;
    LSTATUS status = RegOpenKeyExW(HKEY_LOCAL_MACHINE, path.c_str(), 0, KEY_SET_VALUE | KEY_WOW64_64KEY, &key);
    if (status != ERROR_SUCCESS) return status;
    status = RegDeleteValueW(key, name);
    RegCloseKey(key);
    return status;
}

int Show(const Endpoint& endpoint) {
    const std::wstring fx = FxPath(endpoint);
    std::wstring efx;
    DWORD disabled = 0;
    const bool hasEfx = ReadRegString(fx, kEfxValue, efx);
    const bool hasDisabled = ReadRegDword(fx, kDisableSysFxValue, disabled);
    std::wcout << L"ENDPOINT\t" << endpoint.name << L"\t" << endpoint.guid << L"\n";
    std::wcout << L"EFX\t" << (hasEfx ? efx : L"<absent>") << L"\n";
    std::wcout << L"ENHANCEMENTS_DISABLED\t" << (hasDisabled ? disabled : 0) << L"\n";
    return hasEfx && _wcsicmp(efx.c_str(), GuidText(kOmniphonyApoClsid).c_str()) == 0 ? 0 : 3;
}

int Attach(const Endpoint& endpoint) {
    const std::wstring fx = FxPath(endpoint);
    const std::wstring ours = GuidText(kOmniphonyApoClsid);
    std::wstring existing;
    if (ReadRegString(fx, kEfxValue, existing) && _wcsicmp(existing.c_str(), ours.c_str()) != 0) {
        std::wcerr << L"ERROR\tEXISTING_EFX\t" << existing << L"\n";
        return 8;
    }

    std::wstring modes;
    const bool modesExisted = ReadRegString(fx, kEfxModesValue, modes);
    const std::wstring state = std::wstring(kStateBase) + endpoint.guid;
    LSTATUS status = WriteRegDword(state, L"ModesExisted", modesExisted ? 1u : 0u);
    if (status != ERROR_SUCCESS) {
        std::wcerr << L"ERROR\tSTATE_WRITE\t" << status << L"\n";
        return 5;
    }

    status = WriteRegString(fx, kEfxValue, ours);
    if (status != ERROR_SUCCESS) {
        std::wcerr << L"ERROR\tFX_WRITE\t" << status << L"\n";
        return status == ERROR_ACCESS_DENIED ? 5 : 6;
    }
    if (!modesExisted) {
        status = WriteDefaultMode(fx);
        if (status != ERROR_SUCCESS) {
            DeleteValue(fx, kEfxValue);
            std::wcerr << L"ERROR\tMODE_WRITE\t" << status << L"\n";
            return 6;
        }
    }

    std::wcout << L"APO_ATTACHED\t" << endpoint.name << L"\t" << endpoint.guid << L"\n";
    std::wcout << L"RESTART_AUDIO_REQUIRED\t1\n";
    return 0;
}

int Detach(const Endpoint& endpoint) {
    const std::wstring fx = FxPath(endpoint);
    const std::wstring ours = GuidText(kOmniphonyApoClsid);
    std::wstring existing;
    if (ReadRegString(fx, kEfxValue, existing) && _wcsicmp(existing.c_str(), ours.c_str()) == 0) {
        const LSTATUS status = DeleteValue(fx, kEfxValue);
        if (status != ERROR_SUCCESS && status != ERROR_FILE_NOT_FOUND) {
            std::wcerr << L"ERROR\tFX_DELETE\t" << status << L"\n";
            return 6;
        }
    }

    const std::wstring state = std::wstring(kStateBase) + endpoint.guid;
    DWORD modesExisted = 1;
    if (ReadRegDword(state, L"ModesExisted", modesExisted) && modesExisted == 0) {
        DeleteValue(fx, kEfxModesValue);
    }
    RegDeleteTreeW(HKEY_LOCAL_MACHINE, state.c_str());
    std::wcout << L"APO_DETACHED\t" << endpoint.name << L"\t" << endpoint.guid << L"\n";
    std::wcout << L"RESTART_AUDIO_REQUIRED\t1\n";
    return 0;
}

} // namespace

int wmain(int argc, wchar_t** argv) {
    if (argc < 2) {
        std::wcerr << L"usage: OmniphonyApoCtl <list|status|attach|detach> [endpoint-name-fragments...]\n";
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
    if (!FindEndpoint(Needles(argc, argv, 2), endpoint)) {
        std::wcerr << L"ERROR\tENDPOINT_NOT_FOUND\n";
        CoUninitialize();
        return 3;
    }

    int result = 2;
    if (command == L"status") result = Show(endpoint);
    else if (command == L"attach") result = Attach(endpoint);
    else if (command == L"detach") result = Detach(endpoint);
    else std::wcerr << L"ERROR\tUNKNOWN_COMMAND\n";

    CoUninitialize();
    return result;
}
