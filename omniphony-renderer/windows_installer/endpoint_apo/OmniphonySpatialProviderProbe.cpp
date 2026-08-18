#include <windows.h>
#include <spatialaudioclient.h>

#include <cwchar>
#include <iomanip>
#include <iostream>
#include <string>
#include <vector>

namespace {

constexpr wchar_t kEncoderPath[] =
    L"SOFTWARE\\Microsoft\\Multimedia\\Audio\\Spatial\\Encoder";
constexpr wchar_t kSpatialEndpointPath[] =
    L"SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\MMDevices\\SpatialAudioEndpoint";

std::wstring Win32Message(LONG code) {
    wchar_t buffer[512] = {};
    const DWORD count = FormatMessageW(
        FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_IGNORE_INSERTS,
        nullptr,
        static_cast<DWORD>(code),
        0,
        buffer,
        static_cast<DWORD>(sizeof(buffer) / sizeof(buffer[0])),
        nullptr);
    if (count == 0) {
        return L"unknown";
    }
    std::wstring message(buffer, count);
    while (!message.empty() &&
           (message.back() == L'\r' || message.back() == L'\n' || message.back() == L' ')) {
        message.pop_back();
    }
    return message;
}

std::wstring HResultText(HRESULT hr) {
    wchar_t buffer[16] = {};
    swprintf_s(buffer, L"0x%08lX", static_cast<unsigned long>(hr));
    return buffer;
}

std::wstring ReadStringValue(HKEY key, const wchar_t* valueName) {
    DWORD bytes = 0;
    LONG result = RegGetValueW(
        key,
        nullptr,
        valueName,
        RRF_RT_REG_SZ | RRF_RT_REG_EXPAND_SZ,
        nullptr,
        nullptr,
        &bytes);
    if (result != ERROR_SUCCESS || bytes == 0) {
        return {};
    }

    std::vector<wchar_t> value((bytes / sizeof(wchar_t)) + 1, L'\0');
    result = RegGetValueW(
        key,
        nullptr,
        valueName,
        RRF_RT_REG_SZ | RRF_RT_REG_EXPAND_SZ,
        nullptr,
        value.data(),
        &bytes);
    if (result != ERROR_SUCCESS) {
        return {};
    }
    return value.data();
}

void PrintStringField(const std::wstring& prefix, const wchar_t* field, const std::wstring& value) {
    std::wcout << prefix << L"." << field << L"=";
    if (value.empty()) {
        std::wcout << L"<missing>";
    } else {
        std::wcout << value;
    }
    std::wcout << L"\n";
}

LONG OpenReadOnly64(const wchar_t* path, HKEY* key) {
    return RegOpenKeyExW(
        HKEY_LOCAL_MACHINE,
        path,
        0,
        KEY_READ | KEY_WOW64_64KEY,
        key);
}

void ProbeProviderCom(const std::wstring& prefix, const std::wstring& clsidText) {
    std::wcout << prefix << L".direct_com.context=CLSCTX_INPROC_SERVER\n";
    std::wcout << prefix << L".direct_com.hypothesis=encoder_clsid_exposes_ispatialaudioclient\n";

    if (clsidText.empty()) {
        std::wcout << prefix << L".direct_com.status=no_clsid\n";
        return;
    }

    CLSID clsid = {};
    const HRESULT parseHr = CLSIDFromString(clsidText.c_str(), &clsid);
    if (FAILED(parseHr)) {
        std::wcout << prefix << L".direct_com.status=invalid_clsid\n";
        std::wcout << prefix << L".direct_com.parse_hr=" << HResultText(parseHr) << L"\n";
        return;
    }

    ISpatialAudioClient* client = nullptr;
    const HRESULT activateHr = CoCreateInstance(
        clsid,
        nullptr,
        CLSCTX_INPROC_SERVER,
        __uuidof(ISpatialAudioClient),
        reinterpret_cast<void**>(&client));
    std::wcout << prefix << L".direct_com.activate_hr=" << HResultText(activateHr) << L"\n";
    if (FAILED(activateHr) || !client) {
        std::wcout << prefix << L".direct_com.status=not_direct_ispatialaudioclient\n";
        return;
    }

    std::wcout << prefix << L".direct_com.status=ispatialaudioclient\n";

    AudioObjectType staticMask = {};
    const HRESULT maskHr = client->GetNativeStaticObjectTypeMask(&staticMask);
    std::wcout << prefix << L".direct_com.static_mask_hr=" << HResultText(maskHr) << L"\n";
    if (SUCCEEDED(maskHr)) {
        std::wcout << prefix << L".direct_com.static_mask=0x"
                   << std::hex << std::uppercase << static_cast<unsigned long>(staticMask)
                   << std::dec << std::nouppercase << L"\n";
    }

    UINT32 maxDynamicObjects = 0;
    const HRESULT dynamicHr = client->GetMaxDynamicObjectCount(&maxDynamicObjects);
    std::wcout << prefix << L".direct_com.max_dynamic_hr=" << HResultText(dynamicHr) << L"\n";
    if (SUCCEEDED(dynamicHr)) {
        std::wcout << prefix << L".direct_com.max_dynamic_objects=" << maxDynamicObjects << L"\n";
    }

    client->Release();
}

void ProbeEncoderRegistry(bool probeDirectCom) {
    std::wcout << L"encoder.path=HKLM\\" << kEncoderPath << L"\n";

    HKEY root = nullptr;
    const LONG openResult = OpenReadOnly64(kEncoderPath, &root);
    if (openResult != ERROR_SUCCESS) {
        std::wcout << L"encoder.status=unavailable\n";
        std::wcout << L"encoder.error=" << openResult << L":" << Win32Message(openResult) << L"\n";
        return;
    }

    std::wcout << L"encoder.status=available\n";

    DWORD subkeyCount = 0;
    DWORD maxSubkeyLength = 0;
    const LONG infoResult = RegQueryInfoKeyW(
        root,
        nullptr,
        nullptr,
        nullptr,
        &subkeyCount,
        &maxSubkeyLength,
        nullptr,
        nullptr,
        nullptr,
        nullptr,
        nullptr,
        nullptr);
    if (infoResult != ERROR_SUCCESS) {
        std::wcout << L"encoder.enumeration_error=" << infoResult << L":" << Win32Message(infoResult) << L"\n";
        RegCloseKey(root);
        return;
    }

    std::wcout << L"encoder.count=" << subkeyCount << L"\n";
    std::vector<wchar_t> name(maxSubkeyLength + 2, L'\0');

    for (DWORD index = 0; index < subkeyCount; ++index) {
        DWORD nameLength = static_cast<DWORD>(name.size());
        FILETIME lastWrite = {};
        const LONG enumResult = RegEnumKeyExW(
            root,
            index,
            name.data(),
            &nameLength,
            nullptr,
            nullptr,
            nullptr,
            &lastWrite);
        if (enumResult != ERROR_SUCCESS) {
            std::wcout << L"encoder[" << index << L"].error="
                       << enumResult << L":" << Win32Message(enumResult) << L"\n";
            continue;
        }

        const std::wstring formatGuid(name.data(), nameLength);
        const std::wstring prefix = L"encoder[" + std::to_wstring(index) + L"]";
        std::wcout << prefix << L".format_guid=" << formatGuid << L"\n";

        HKEY provider = nullptr;
        const LONG providerResult = RegOpenKeyExW(
            root,
            formatGuid.c_str(),
            0,
            KEY_READ,
            &provider);
        if (providerResult != ERROR_SUCCESS) {
            std::wcout << prefix << L".open_error="
                       << providerResult << L":" << Win32Message(providerResult) << L"\n";
            continue;
        }

        const std::wstring displayName = ReadStringValue(provider, nullptr);
        const std::wstring clsidText = ReadStringValue(provider, L"CLSID");
        const std::wstring iconPath = ReadStringValue(provider, L"IconPath");
        PrintStringField(prefix, L"display_name", displayName);
        PrintStringField(prefix, L"clsid", clsidText);
        PrintStringField(prefix, L"icon_path", iconPath);
        RegCloseKey(provider);

        if (probeDirectCom) {
            ProbeProviderCom(prefix, clsidText);
        }
    }

    RegCloseKey(root);
}

void ProbeSpatialEndpointRegistry() {
    std::wcout << L"spatial_endpoint.path=HKLM\\" << kSpatialEndpointPath << L"\n";

    HKEY root = nullptr;
    const LONG openResult = OpenReadOnly64(kSpatialEndpointPath, &root);
    if (openResult != ERROR_SUCCESS) {
        std::wcout << L"spatial_endpoint.status=unavailable\n";
        std::wcout << L"spatial_endpoint.error=" << openResult << L":" << Win32Message(openResult) << L"\n";
        return;
    }

    std::wcout << L"spatial_endpoint.status=available\n";

    DWORD subkeyCount = 0;
    DWORD maxSubkeyLength = 0;
    const LONG infoResult = RegQueryInfoKeyW(
        root,
        nullptr,
        nullptr,
        nullptr,
        &subkeyCount,
        &maxSubkeyLength,
        nullptr,
        nullptr,
        nullptr,
        nullptr,
        nullptr,
        nullptr);
    if (infoResult != ERROR_SUCCESS) {
        std::wcout << L"spatial_endpoint.enumeration_error="
                   << infoResult << L":" << Win32Message(infoResult) << L"\n";
        RegCloseKey(root);
        return;
    }

    std::wcout << L"spatial_endpoint.subkey_count=" << subkeyCount << L"\n";
    std::vector<wchar_t> name(maxSubkeyLength + 2, L'\0');
    for (DWORD index = 0; index < subkeyCount; ++index) {
        DWORD nameLength = static_cast<DWORD>(name.size());
        FILETIME lastWrite = {};
        const LONG enumResult = RegEnumKeyExW(
            root,
            index,
            name.data(),
            &nameLength,
            nullptr,
            nullptr,
            nullptr,
            &lastWrite);
        if (enumResult == ERROR_SUCCESS) {
            std::wcout << L"spatial_endpoint.subkey[" << index << L"]="
                       << std::wstring(name.data(), nameLength) << L"\n";
        } else {
            std::wcout << L"spatial_endpoint.subkey[" << index << L"].error="
                       << enumResult << L":" << Win32Message(enumResult) << L"\n";
        }
    }

    RegCloseKey(root);
}

} // namespace

int wmain(int argc, wchar_t** argv) {
    bool probeDirectCom = false;
    if (argc == 2 && std::wcscmp(argv[1], L"--probe-com") == 0) {
        probeDirectCom = true;
    } else if (argc != 1) {
        std::wcerr << L"usage=OmniphonySpatialProviderProbe.exe [--probe-com]\n";
        return 2;
    }

    HRESULT comHr = S_OK;
    bool uninitializeCom = false;
    if (probeDirectCom) {
        comHr = CoInitializeEx(nullptr, COINIT_MULTITHREADED);
        if (FAILED(comHr)) {
            std::wcerr << L"direct_com.initialize_hr=" << HResultText(comHr) << L"\n";
            return 3;
        }
        uninitializeCom = true;
    }

    std::wcout << L"probe=omniphony_spatial_provider\n";
    std::wcout << L"mode="
               << (probeDirectCom ? L"read_only_observation_plus_com_canary" : L"read_only_observation")
               << L"\n";
    std::wcout << L"source_truth=undocumented_registry_surface_not_public_api_contract\n";
    std::wcout << L"direct_com_canary=" << (probeDirectCom ? 1 : 0) << L"\n";
    if (probeDirectCom) {
        std::wcout << L"direct_com_scope=tests_only_encoder_clsid_to_ispatialaudioclient_hypothesis\n";
        std::wcout << L"direct_com_nonclaim=does_not_prove_windows_provider_selection_or_object_delivery\n";
    }

    ProbeEncoderRegistry(probeDirectCom);
    ProbeSpatialEndpointRegistry();

    if (uninitializeCom) {
        CoUninitialize();
    }
    return 0;
}
