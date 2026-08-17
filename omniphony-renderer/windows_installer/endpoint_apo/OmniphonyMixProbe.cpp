// WIN32_LEAN_AND_MEAN and NOMINMAX are supplied by CMake so /WX sees no macro redefinitions.
#include <windows.h>
#include <audioclient.h>
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

HRESULT FriendlyName(IMMDevice* device, std::wstring& name) {
    ComPtr<IPropertyStore> store;
    HRESULT hr = device->OpenPropertyStore(STGM_READ, store.ReleaseAndGetAddressOf());
    if (FAILED(hr)) {
        return hr;
    }

    PROPVARIANT value;
    PropVariantInit(&value);
    hr = store->GetValue(PKEY_Device_FriendlyName, &value);
    if (SUCCEEDED(hr)) {
        if (value.vt == VT_LPWSTR && value.pwszVal) {
            name.assign(value.pwszVal);
        } else {
            hr = E_UNEXPECTED;
        }
    }
    PropVariantClear(&value);
    return hr;
}

HRESULT FindRenderEndpoint(const std::vector<std::wstring>& needles, ComPtr<IMMDevice>& device, std::wstring& name) {
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
                device = candidate;
                name = std::move(candidateName);
                return S_OK;
            }
        }
    }
    return HRESULT_FROM_WIN32(ERROR_NOT_FOUND);
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
        HRESULT hr = FindRenderEndpoint(needles, device, name);
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
