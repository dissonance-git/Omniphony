#include <windows.h>
#include <spatialaudioclient.h>

#include <algorithm>
#include <cwchar>
#include <iomanip>
#include <iostream>
#include <sstream>
#include <string>
#include <vector>

namespace {

constexpr wchar_t kEncoderPath[] =
    L"SOFTWARE\\Microsoft\\Multimedia\\Audio\\Spatial\\Encoder";
constexpr wchar_t kSpatialEndpointPath[] =
    L"SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\MMDevices\\SpatialAudioEndpoint";
constexpr DWORD kSnapshotMaxDepth = 8;
constexpr DWORD kSnapshotMaxValueBytes = 4096;

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

std::wstring RegistryTypeName(DWORD type) {
    switch (type) {
        case REG_NONE:
            return L"REG_NONE";
        case REG_SZ:
            return L"REG_SZ";
        case REG_EXPAND_SZ:
            return L"REG_EXPAND_SZ";
        case REG_BINARY:
            return L"REG_BINARY";
        case REG_DWORD:
            return L"REG_DWORD";
        case REG_DWORD_BIG_ENDIAN:
            return L"REG_DWORD_BIG_ENDIAN";
        case REG_LINK:
            return L"REG_LINK";
        case REG_MULTI_SZ:
            return L"REG_MULTI_SZ";
        case REG_RESOURCE_LIST:
            return L"REG_RESOURCE_LIST";
        case REG_FULL_RESOURCE_DESCRIPTOR:
            return L"REG_FULL_RESOURCE_DESCRIPTOR";
        case REG_RESOURCE_REQUIREMENTS_LIST:
            return L"REG_RESOURCE_REQUIREMENTS_LIST";
        case REG_QWORD:
            return L"REG_QWORD";
        default:
            return L"REG_UNKNOWN";
    }
}

std::wstring BytesToHex(const std::vector<BYTE>& bytes) {
    std::wostringstream out;
    out << std::hex << std::uppercase << std::setfill(L'0');
    for (BYTE value : bytes) {
        out << std::setw(2) << static_cast<unsigned int>(value);
    }
    return out.str();
}

std::vector<std::wstring> SortedSubkeyNames(HKEY key, const std::wstring& prefix) {
    DWORD subkeyCount = 0;
    DWORD maxSubkeyLength = 0;
    const LONG infoResult = RegQueryInfoKeyW(
        key,
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
        std::wcout << prefix << L".subkey_query_error="
                   << infoResult << L":" << Win32Message(infoResult) << L"\n";
        return {};
    }

    std::vector<std::wstring> names;
    names.reserve(subkeyCount);
    std::vector<wchar_t> name(maxSubkeyLength + 2, L'\0');
    for (DWORD index = 0; index < subkeyCount; ++index) {
        DWORD nameLength = static_cast<DWORD>(name.size());
        FILETIME lastWrite = {};
        const LONG enumResult = RegEnumKeyExW(
            key,
            index,
            name.data(),
            &nameLength,
            nullptr,
            nullptr,
            nullptr,
            &lastWrite);
        if (enumResult == ERROR_SUCCESS) {
            names.emplace_back(name.data(), nameLength);
        } else {
            std::wcout << prefix << L".subkey_enum_error[" << index << L"]="
                       << enumResult << L":" << Win32Message(enumResult) << L"\n";
        }
    }
    std::sort(names.begin(), names.end());
    return names;
}

std::vector<std::wstring> SortedValueNames(HKEY key, const std::wstring& prefix) {
    DWORD valueCount = 0;
    DWORD maxValueNameLength = 0;
    const LONG infoResult = RegQueryInfoKeyW(
        key,
        nullptr,
        nullptr,
        nullptr,
        nullptr,
        nullptr,
        nullptr,
        &valueCount,
        &maxValueNameLength,
        nullptr,
        nullptr,
        nullptr);
    if (infoResult != ERROR_SUCCESS) {
        std::wcout << prefix << L".value_query_error="
                   << infoResult << L":" << Win32Message(infoResult) << L"\n";
        return {};
    }

    std::vector<std::wstring> names;
    names.reserve(valueCount);
    std::vector<wchar_t> name(maxValueNameLength + 2, L'\0');
    for (DWORD index = 0; index < valueCount; ++index) {
        DWORD nameLength = static_cast<DWORD>(name.size());
        const LONG enumResult = RegEnumValueW(
            key,
            index,
            name.data(),
            &nameLength,
            nullptr,
            nullptr,
            nullptr,
            nullptr);
        if (enumResult == ERROR_SUCCESS) {
            names.emplace_back(name.data(), nameLength);
        } else {
            std::wcout << prefix << L".value_enum_error[" << index << L"]="
                       << enumResult << L":" << Win32Message(enumResult) << L"\n";
        }
    }
    std::sort(names.begin(), names.end());
    return names;
}

void PrintRegistryValues(HKEY key, const std::wstring& prefix) {
    const auto names = SortedValueNames(key, prefix);
    std::wcout << prefix << L".value_count=" << names.size() << L"\n";

    for (size_t index = 0; index < names.size(); ++index) {
        const std::wstring& name = names[index];
        const wchar_t* queryName = name.empty() ? nullptr : name.c_str();
        const std::wstring valuePrefix =
            prefix + L".value[" + std::to_wstring(index) + L"]";
        std::wcout << valuePrefix << L".name="
                   << (name.empty() ? L"(Default)" : name) << L"\n";

        DWORD type = REG_NONE;
        DWORD byteCount = 0;
        LONG queryResult = RegQueryValueExW(
            key,
            queryName,
            nullptr,
            &type,
            nullptr,
            &byteCount);
        if (queryResult != ERROR_SUCCESS) {
            std::wcout << valuePrefix << L".query_error="
                       << queryResult << L":" << Win32Message(queryResult) << L"\n";
            continue;
        }

        std::wcout << valuePrefix << L".type=" << RegistryTypeName(type)
                   << L"(" << type << L")\n";
        std::wcout << valuePrefix << L".byte_count=" << byteCount << L"\n";

        if (byteCount > kSnapshotMaxValueBytes) {
            std::wcout << valuePrefix << L".truncated=1\n";
            std::wcout << valuePrefix << L".data_hex=<omitted>\n";
            continue;
        }

        std::vector<BYTE> data(byteCount);
        DWORD readBytes = byteCount;
        if (byteCount > 0) {
            queryResult = RegQueryValueExW(
                key,
                queryName,
                nullptr,
                &type,
                data.data(),
                &readBytes);
            if (queryResult != ERROR_SUCCESS) {
                std::wcout << valuePrefix << L".read_error="
                           << queryResult << L":" << Win32Message(queryResult) << L"\n";
                continue;
            }
            data.resize(readBytes);
        }

        std::wcout << valuePrefix << L".truncated=0\n";
        std::wcout << valuePrefix << L".data_hex=" << BytesToHex(data) << L"\n";
    }
}

void PrintRegistryTree(HKEY key, const std::wstring& prefix, DWORD depth) {
    std::wcout << prefix << L".snapshot_depth=" << depth << L"\n";
    PrintRegistryValues(key, prefix);

    const auto names = SortedSubkeyNames(key, prefix);
    std::wcout << prefix << L".snapshot_subkey_count=" << names.size() << L"\n";
    if (depth >= kSnapshotMaxDepth) {
        if (!names.empty()) {
            std::wcout << prefix << L".snapshot_depth_limited=1\n";
        }
        return;
    }

    for (size_t index = 0; index < names.size(); ++index) {
        const std::wstring& name = names[index];
        const std::wstring childPrefix =
            prefix + L".subkey[" + std::to_wstring(index) + L"]";
        std::wcout << childPrefix << L".name=" << name << L"\n";

        HKEY child = nullptr;
        const LONG openResult = RegOpenKeyExW(
            key,
            name.c_str(),
            0,
            KEY_READ,
            &child);
        if (openResult != ERROR_SUCCESS) {
            std::wcout << childPrefix << L".open_error="
                       << openResult << L":" << Win32Message(openResult) << L"\n";
            continue;
        }

        PrintRegistryTree(child, childPrefix, depth + 1);
        RegCloseKey(child);
    }
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
    std::wcout << L"spatial_endpoint.snapshot.read_only=1\n";
    std::wcout << L"spatial_endpoint.snapshot.max_depth=" << kSnapshotMaxDepth << L"\n";
    std::wcout << L"spatial_endpoint.snapshot.max_value_bytes=" << kSnapshotMaxValueBytes << L"\n";
    std::wcout << L"spatial_endpoint.snapshot.encoding=registry_bytes_hex\n";

    HKEY root = nullptr;
    const LONG openResult = OpenReadOnly64(kSpatialEndpointPath, &root);
    if (openResult != ERROR_SUCCESS) {
        std::wcout << L"spatial_endpoint.status=unavailable\n";
        std::wcout << L"spatial_endpoint.error=" << openResult << L":" << Win32Message(openResult) << L"\n";
        return;
    }

    std::wcout << L"spatial_endpoint.status=available\n";
    PrintRegistryTree(root, L"spatial_endpoint.snapshot", 0);
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
