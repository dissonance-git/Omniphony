// WIN32_LEAN_AND_MEAN and NOMINMAX are supplied by CMake so /WX sees no macro redefinitions.
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
#include <iomanip>
#include <iostream>
#include <sstream>
#include <string>
#include <vector>

using Microsoft::WRL::ComPtr;

namespace {

constexpr PROPERTYKEY kEndpointGuid = {
    {0x1da5d803, 0xd492, 0x4edd, {0x8c, 0x23, 0xe0, 0xc0, 0xff, 0xee, 0x7f, 0x0e}}, 4};

constexpr wchar_t kRenderBase[] =
    L"SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\MMDevices\\Audio\\Render\\";
constexpr wchar_t kSfxValue[] = L"{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},5";
constexpr wchar_t kMfxValue[] = L"{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},6";
constexpr wchar_t kCompositeSfxValue[] = L"{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},13";
constexpr wchar_t kCompositeMfxValue[] = L"{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},14";

std::wstring Lower(std::wstring value) {
    std::transform(value.begin(), value.end(), value.begin(), [](wchar_t ch) {
        return static_cast<wchar_t>(std::towlower(ch));
    });
    return value;
}

bool ContainsInsensitive(const std::wstring& haystack, const std::wstring& needle) {
    return !needle.empty() && Lower(haystack).find(Lower(needle)) != std::wstring::npos;
}

std::wstring HResultText(HRESULT hr) {
    std::wostringstream out;
    out << L"0x" << std::uppercase << std::hex << std::setw(8) << std::setfill(L'0')
        << static_cast<unsigned long>(hr);
    return out.str();
}

HRESULT StringProperty(IMMDevice* device, REFPROPERTYKEY key, std::wstring& value) {
    ComPtr<IPropertyStore> store;
    HRESULT hr = device->OpenPropertyStore(STGM_READ, store.ReleaseAndGetAddressOf());
    if (FAILED(hr)) {
        return hr;
    }

    PROPVARIANT property;
    PropVariantInit(&property);
    hr = store->GetValue(key, &property);
    if (SUCCEEDED(hr)) {
        if (property.vt == VT_LPWSTR && property.pwszVal) {
            value.assign(property.pwszVal);
        } else {
            hr = E_UNEXPECTED;
        }
    }
    PropVariantClear(&property);
    return hr;
}

HRESULT FriendlyName(IMMDevice* device, std::wstring& name) {
    return StringProperty(device, PKEY_Device_FriendlyName, name);
}

HRESULT EndpointGuid(IMMDevice* device, std::wstring& guid) {
    return StringProperty(device, kEndpointGuid, guid);
}

HRESULT FindRenderEndpoint(const std::vector<std::wstring>& needles,
                           ComPtr<IMMDevice>& device,
                           std::wstring& name,
                           std::wstring& guid) {
    ComPtr<IMMDeviceEnumerator> enumerator;
    HRESULT hr = CoCreateInstance(
        __uuidof(MMDeviceEnumerator), nullptr, CLSCTX_INPROC_SERVER,
        IID_PPV_ARGS(enumerator.ReleaseAndGetAddressOf()));
    if (FAILED(hr)) {
        return hr;
    }

    ComPtr<IMMDeviceCollection> collection;
    hr = enumerator->EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE, collection.ReleaseAndGetAddressOf());
    if (FAILED(hr)) {
        return hr;
    }

    UINT count = 0;
    hr = collection->GetCount(&count);
    if (FAILED(hr)) {
        return hr;
    }

    for (UINT index = 0; index < count; ++index) {
        ComPtr<IMMDevice> candidate;
        hr = collection->Item(index, candidate.ReleaseAndGetAddressOf());
        if (FAILED(hr)) {
            return hr;
        }
        std::wstring candidateName;
        if (FAILED(FriendlyName(candidate.Get(), candidateName))) {
            continue;
        }
        for (const auto& needle : needles) {
            if (ContainsInsensitive(candidateName, needle)) {
                std::wstring candidateGuid;
                if (FAILED(EndpointGuid(candidate.Get(), candidateGuid))) {
                    return E_UNEXPECTED;
                }
                device = candidate;
                name = std::move(candidateName);
                guid = std::move(candidateGuid);
                return S_OK;
            }
        }
    }
    return HRESULT_FROM_WIN32(ERROR_NOT_FOUND);
}

bool FileExists(const std::wstring& path) {
    if (path.empty()) {
        return false;
    }
    const DWORD attributes = GetFileAttributesW(path.c_str());
    return attributes != INVALID_FILE_ATTRIBUTES && (attributes & FILE_ATTRIBUTE_DIRECTORY) == 0;
}

std::wstring ExpandPath(const std::wstring& input) {
    if (input.empty()) {
        return {};
    }
    DWORD needed = ExpandEnvironmentStringsW(input.c_str(), nullptr, 0);
    if (needed == 0) {
        return input;
    }
    std::vector<wchar_t> buffer(needed, L'\0');
    if (ExpandEnvironmentStringsW(input.c_str(), buffer.data(), needed) == 0) {
        return input;
    }
    std::wstring result(buffer.data());
    if (result.size() >= 2 && result.front() == L'"' && result.back() == L'"') {
        result = result.substr(1, result.size() - 2);
    }
    return result;
}

bool ResolveComServer(const std::wstring& clsid, std::wstring& serverPath) {
    if (clsid.empty()) {
        return false;
    }
    const std::wstring subkey = L"CLSID\\" + clsid + L"\\InprocServer32";
    HKEY key = nullptr;
    if (RegOpenKeyExW(HKEY_CLASSES_ROOT, subkey.c_str(), 0,
                      KEY_QUERY_VALUE | KEY_WOW64_64KEY, &key) != ERROR_SUCCESS) {
        return false;
    }

    DWORD type = 0;
    DWORD size = 0;
    LSTATUS status = RegQueryValueExW(key, nullptr, nullptr, &type, nullptr, &size);
    if (status != ERROR_SUCCESS || (type != REG_SZ && type != REG_EXPAND_SZ) || size == 0) {
        RegCloseKey(key);
        return false;
    }

    std::vector<wchar_t> buffer(size / sizeof(wchar_t) + 2, L'\0');
    status = RegQueryValueExW(key, nullptr, nullptr, &type,
                              reinterpret_cast<BYTE*>(buffer.data()), &size);
    RegCloseKey(key);
    if (status != ERROR_SUCCESS) {
        return false;
    }

    serverPath = ExpandPath(buffer.data());
    return FileExists(serverPath);
}

std::vector<std::wstring> ParseRegistryStrings(DWORD type, const std::vector<wchar_t>& buffer) {
    std::vector<std::wstring> values;
    if (buffer.empty()) {
        return values;
    }
    if (type == REG_SZ) {
        if (buffer[0] != L'\0') {
            values.emplace_back(buffer.data());
        }
        return values;
    }
    if (type != REG_MULTI_SZ) {
        return values;
    }

    const wchar_t* cursor = buffer.data();
    const wchar_t* end = buffer.data() + buffer.size();
    while (cursor < end && *cursor != L'\0') {
        std::wstring value(cursor);
        values.push_back(value);
        cursor += value.size() + 1;
    }
    return values;
}

bool WriteRegistryStrings(HKEY key,
                          const wchar_t* valueName,
                          DWORD type,
                          const std::vector<std::wstring>& values) {
    if (values.empty()) {
        const LSTATUS status = RegDeleteValueW(key, valueName);
        return status == ERROR_SUCCESS || status == ERROR_FILE_NOT_FOUND;
    }

    if (type == REG_SZ) {
        const std::wstring& value = values.front();
        return RegSetValueExW(
                   key, valueName, 0, REG_SZ,
                   reinterpret_cast<const BYTE*>(value.c_str()),
                   static_cast<DWORD>((value.size() + 1) * sizeof(wchar_t))) == ERROR_SUCCESS;
    }

    std::vector<wchar_t> data;
    for (const auto& value : values) {
        data.insert(data.end(), value.begin(), value.end());
        data.push_back(L'\0');
    }
    data.push_back(L'\0');
    return RegSetValueExW(
               key, valueName, 0, REG_MULTI_SZ,
               reinterpret_cast<const BYTE*>(data.data()),
               static_cast<DWORD>(data.size() * sizeof(wchar_t))) == ERROR_SUCCESS;
}

int RepairValue(HKEY key, const wchar_t* valueName) {
    DWORD type = 0;
    DWORD size = 0;
    LSTATUS status = RegQueryValueExW(key, valueName, nullptr, &type, nullptr, &size);
    if (status == ERROR_FILE_NOT_FOUND) {
        return 0;
    }
    if (status != ERROR_SUCCESS || (type != REG_SZ && type != REG_MULTI_SZ) || size == 0) {
        return 0;
    }

    std::vector<wchar_t> buffer(size / sizeof(wchar_t) + 2, L'\0');
    status = RegQueryValueExW(key, valueName, nullptr, &type,
                              reinterpret_cast<BYTE*>(buffer.data()), &size);
    if (status != ERROR_SUCCESS) {
        return 0;
    }

    const auto values = ParseRegistryStrings(type, buffer);
    if (values.empty()) {
        return 0;
    }

    std::vector<std::wstring> keep;
    int removed = 0;
    for (const auto& clsid : values) {
        std::wstring server;
        if (ResolveComServer(clsid, server)) {
            keep.push_back(clsid);
            std::wcout << L"FX_SERVER_OK\t" << valueName << L'\t' << clsid
                       << L'\t' << server << L'\n';
        } else {
            ++removed;
            std::wcerr << L"STALE_FX_MISSING_SERVER\t" << valueName << L'\t'
                       << clsid << L'\n';
        }
    }

    if (removed > 0) {
        if (!WriteRegistryStrings(key, valueName, type, keep)) {
            std::wcerr << L"STALE_FX_REPAIR_WRITE_FAILED\t" << valueName << L'\n';
            return 0;
        }
        std::wcout << L"STALE_FX_REPAIRED\t" << valueName << L"\tREMOVED=" << removed << L'\n';
    }
    return removed;
}

int RepairMissingEffects(const std::wstring& endpointGuid) {
    const std::wstring path = std::wstring(kRenderBase) + endpointGuid + L"\\FxProperties";
    HKEY key = nullptr;
    const LSTATUS status = RegOpenKeyExW(
        HKEY_LOCAL_MACHINE, path.c_str(), 0,
        KEY_QUERY_VALUE | KEY_SET_VALUE | KEY_WOW64_64KEY, &key);
    if (status != ERROR_SUCCESS) {
        std::wcerr << L"STALE_FX_REPAIR_OPEN_FAILED\t" << status << L'\n';
        return 0;
    }

    int removed = 0;
    removed += RepairValue(key, kSfxValue);
    removed += RepairValue(key, kMfxValue);
    removed += RepairValue(key, kCompositeSfxValue);
    removed += RepairValue(key, kCompositeMfxValue);
    RegCloseKey(key);
    return removed;
}

} // namespace

int wmain(int argc, wchar_t** argv) {
    if (argc < 2) {
        std::wcerr << L"usage: OmniphonyMixProbe <endpoint-name-needle> [more needles...]\n";
        return 2;
    }

    const HRESULT init = CoInitializeEx(nullptr, COINIT_MULTITHREADED);
    if (FAILED(init)) {
        std::wcerr << L"MIX_PROBE_COM_FAILED\t" << HResultText(init) << L'\n';
        return 3;
    }

    int result = 0;
    {
        std::vector<std::wstring> needles;
        for (int i = 1; i < argc; ++i) {
            if (argv[i] && *argv[i]) {
                needles.emplace_back(argv[i]);
            }
        }

        ComPtr<IMMDevice> device;
        std::wstring name;
        std::wstring guid;
        HRESULT hr = FindRenderEndpoint(needles, device, name, guid);
        if (FAILED(hr)) {
            std::wcerr << L"MIX_PROBE_ENDPOINT_FAILED\t" << HResultText(hr) << L'\n';
            result = 4;
        } else {
            ComPtr<IAudioClient> client;
            hr = device->Activate(
                __uuidof(IAudioClient), CLSCTX_ALL, nullptr,
                reinterpret_cast<void**>(client.ReleaseAndGetAddressOf()));
            if (FAILED(hr)) {
                std::wcerr << L"MIX_PROBE_ACTIVATE_FAILED\t" << name << L'\t'
                           << HResultText(hr) << L'\n';
                result = 5;
            } else {
                WAVEFORMATEX* format = nullptr;
                hr = client->GetMixFormat(&format);
                if (FAILED(hr)) {
                    std::wcerr << L"MIX_PROBE_GETMIXFORMAT_FAILED\t" << name << L'\t'
                               << HResultText(hr) << L'\n';
                    if (hr == HRESULT_FROM_WIN32(ERROR_MOD_NOT_FOUND)) {
                        const int removed = RepairMissingEffects(guid);
                        if (removed > 0) {
                            std::wcout << L"STALE_FX_REPAIR_COUNT\t" << removed << L'\n';
                        }
                    }
                    result = 6;
                } else if (!format) {
                    std::wcerr << L"MIX_PROBE_NULL_FORMAT\t" << name << L'\n';
                    result = 7;
                } else {
                    std::wcout << L"MIX_FORMAT_OK\t" << name
                               << L"\tRATE=" << format->nSamplesPerSec
                               << L"\tCHANNELS=" << format->nChannels
                               << L"\tBITS=" << format->wBitsPerSample
                               << L"\tTAG=0x" << std::hex << format->wFormatTag << std::dec
                               << L'\n';
                }
                if (format) {
                    CoTaskMemFree(format);
                }
            }
        }
    }

    CoUninitialize();
    return result;
}
