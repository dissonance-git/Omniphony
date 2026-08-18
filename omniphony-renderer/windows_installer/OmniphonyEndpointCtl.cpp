#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>
#include <audioclient.h>
#include <propkeydef.h>
#include <functiondiscoverykeys_devpkey.h>
#include <mmdeviceapi.h>
#include <newdev.h>
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

// Windows exposes no public API for changing the system default audio endpoint.
// This Windows-only adapter uses the long-lived PolicyConfig COM ABI also used
// by mature Windows audio projects. Keep it below the portable renderer boundary.

using Microsoft::WRL::ComPtr;

struct DeviceShareMode;

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

constexpr int kExitUsage = 2;
constexpr int kExitNotFound = 3;
constexpr int kExitCom = 4;
constexpr int kExitVerify = 5;
constexpr int kExitEnumeration = 6;
constexpr int kExitDriver = 7;

struct Endpoint {
    std::wstring id;
    std::wstring name;
};

class ComApartment {
public:
    ComApartment() : hr_(CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED)), owns_(SUCCEEDED(hr_)) {}
    ~ComApartment() {
        if (owns_) CoUninitialize();
    }
    HRESULT status() const { return hr_; }
private:
    HRESULT hr_;
    bool owns_;
};

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
    wchar_t* buffer = nullptr;
    const DWORD flags = FORMAT_MESSAGE_ALLOCATE_BUFFER | FORMAT_MESSAGE_FROM_SYSTEM |
                        FORMAT_MESSAGE_IGNORE_INSERTS;
    const DWORD count = FormatMessageW(
        flags, nullptr, static_cast<DWORD>(hr),
        MAKELANGID(LANG_NEUTRAL, SUBLANG_DEFAULT),
        reinterpret_cast<wchar_t*>(&buffer), 0, nullptr);

    std::wostringstream out;
    out << L"0x" << std::uppercase << std::hex << std::setw(8) << std::setfill(L'0')
        << static_cast<unsigned long>(hr);
    if (count && buffer) {
        std::wstring message(buffer, count);
        while (!message.empty() && (message.back() == L'\r' || message.back() == L'\n')) {
            message.pop_back();
        }
        out << L" (" << message << L")";
    }
    if (buffer) LocalFree(buffer);
    return out.str();
}

std::wstring Win32Text(DWORD error) {
    return HResultText(HRESULT_FROM_WIN32(error));
}

HRESULT CreateEnumerator(ComPtr<IMMDeviceEnumerator>& enumerator) {
    return CoCreateInstance(
        __uuidof(MMDeviceEnumerator), nullptr, CLSCTX_INPROC_SERVER,
        IID_PPV_ARGS(enumerator.ReleaseAndGetAddressOf()));
}

HRESULT FriendlyName(IMMDevice* device, std::wstring& name) {
    ComPtr<IPropertyStore> store;
    HRESULT hr = device->OpenPropertyStore(STGM_READ, store.ReleaseAndGetAddressOf());
    if (FAILED(hr)) return hr;

    PROPVARIANT value;
    PropVariantInit(&value);
    hr = store->GetValue(PKEY_Device_FriendlyName, &value);
    if (SUCCEEDED(hr)) {
        if (value.vt == VT_LPWSTR && value.pwszVal) name.assign(value.pwszVal);
        else hr = E_UNEXPECTED;
    }
    PropVariantClear(&value);
    return hr;
}

HRESULT DeviceIdentity(IMMDevice* device, Endpoint& endpoint) {
    LPWSTR rawId = nullptr;
    HRESULT hr = device->GetId(&rawId);
    if (FAILED(hr)) return hr;
    endpoint.id.assign(rawId ? rawId : L"");
    CoTaskMemFree(rawId);

    std::wstring name;
    hr = FriendlyName(device, name);
    if (FAILED(hr)) return hr;
    endpoint.name = std::move(name);
    return S_OK;
}

HRESULT EnumerateRenderEndpoints(std::vector<Endpoint>& endpoints) {
    ComPtr<IMMDeviceEnumerator> enumerator;
    HRESULT hr = CreateEnumerator(enumerator);
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
        hr = collection->Item(index, device.ReleaseAndGetAddressOf());
        if (FAILED(hr)) return hr;
        Endpoint endpoint;
        hr = DeviceIdentity(device.Get(), endpoint);
        if (SUCCEEDED(hr)) endpoints.push_back(std::move(endpoint));
    }
    return S_OK;
}

HRESULT FindByName(const std::vector<std::wstring>& needles, Endpoint& endpoint) {
    std::vector<Endpoint> endpoints;
    HRESULT hr = EnumerateRenderEndpoints(endpoints);
    if (FAILED(hr)) return hr;

    for (const auto& candidate : endpoints) {
        for (const auto& needle : needles) {
            if (ContainsInsensitive(candidate.name, needle)) {
                endpoint = candidate;
                return S_OK;
            }
        }
    }
    return HRESULT_FROM_WIN32(ERROR_NOT_FOUND);
}

HRESULT CreatePolicyConfig(ComPtr<IPolicyConfig>& policy) {
    return CoCreateInstance(
        __uuidof(CPolicyConfigClient), nullptr, CLSCTX_ALL,
        __uuidof(IPolicyConfig), reinterpret_cast<void**>(policy.ReleaseAndGetAddressOf()));
}

HRESULT DefaultId(IMMDeviceEnumerator* enumerator, ERole role, std::wstring& id) {
    ComPtr<IMMDevice> device;
    HRESULT hr = enumerator->GetDefaultAudioEndpoint(eRender, role, device.ReleaseAndGetAddressOf());
    if (FAILED(hr)) return hr;
    LPWSTR rawId = nullptr;
    hr = device->GetId(&rawId);
    if (FAILED(hr)) return hr;
    id.assign(rawId ? rawId : L"");
    CoTaskMemFree(rawId);
    return S_OK;
}

HRESULT VerifyDefault(const std::wstring& expectedId, ERole role) {
    ComPtr<IMMDeviceEnumerator> enumerator;
    HRESULT hr = CreateEnumerator(enumerator);
    if (FAILED(hr)) return hr;

    for (int attempt = 0; attempt < 40; ++attempt) {
        std::wstring actual;
        hr = DefaultId(enumerator.Get(), role, actual);
        if (SUCCEEDED(hr) && _wcsicmp(actual.c_str(), expectedId.c_str()) == 0) return S_OK;
        Sleep(50);
    }
    return HRESULT_FROM_WIN32(ERROR_RETRY);
}

HRESULT SetDefault(const std::wstring& endpointId, bool verify) {
    ComPtr<IPolicyConfig> policy;
    HRESULT hr = CreatePolicyConfig(policy);
    if (FAILED(hr)) return hr;

    const ERole roles[] = {eConsole, eMultimedia, eCommunications};
    for (ERole role : roles) {
        hr = policy->SetDefaultEndpoint(endpointId.c_str(), role);
        if (FAILED(hr)) return hr;
    }

    if (verify) {
        for (ERole role : roles) {
            hr = VerifyDefault(endpointId, role);
            if (FAILED(hr)) return hr;
        }
    }
    return S_OK;
}

int PrintEndpoint(const wchar_t* tag, const Endpoint& endpoint) {
    std::wcout << tag << L'\t' << endpoint.id << L'\t' << endpoint.name << L'\n';
    return 0;
}

int Fail(const wchar_t* context, HRESULT hr, int code) {
    std::wcerr << L"ERROR\t" << context << L'\t' << HResultText(hr) << L'\n';
    return code;
}

std::vector<std::wstring> Needles(int argc, wchar_t** argv, int start) {
    std::vector<std::wstring> needles;
    for (int i = start; i < argc; ++i) {
        if (argv[i] && *argv[i]) needles.emplace_back(argv[i]);
    }
    return needles;
}

const wchar_t* RoleName(ERole role) {
    switch (role) {
        case eConsole: return L"console";
        case eMultimedia: return L"multimedia";
        case eCommunications: return L"communications";
        default: return L"unknown";
    }
}

HRESULT EndpointForRole(IMMDeviceEnumerator* enumerator, ERole role, Endpoint& endpoint) {
    ComPtr<IMMDevice> device;
    HRESULT hr = enumerator->GetDefaultAudioEndpoint(eRender, role, device.ReleaseAndGetAddressOf());
    if (FAILED(hr)) return hr;
    return DeviceIdentity(device.Get(), endpoint);
}

int GetDefaultCommand() {
    ComPtr<IMMDeviceEnumerator> enumerator;
    HRESULT hr = CreateEnumerator(enumerator);
    if (FAILED(hr)) return Fail(L"IMMDeviceEnumerator", hr, kExitEnumeration);

    // AudioSrv can report Running a little before endpoint-role state is fully
    // repopulated. Retry the public default lookup before treating E_NOTFOUND as
    // durable, and consider every supported render role rather than eConsole only.
    const ERole roles[] = {eConsole, eCommunications, eMultimedia};
    HRESULT lastDefaultHr = HRESULT_FROM_WIN32(ERROR_NOT_FOUND);
    for (int attempt = 0; attempt < 20; ++attempt) {
        for (ERole role : roles) {
            Endpoint endpoint;
            hr = EndpointForRole(enumerator.Get(), role, endpoint);
            if (SUCCEEDED(hr)) {
                std::wcout << L"DEFAULT_RESOLUTION\trole=" << RoleName(role)
                           << L"\tattempt=" << attempt + 1 << L'\n';
                return PrintEndpoint(L"DEFAULT", endpoint);
            }
            lastDefaultHr = hr;
        }
        Sleep(100);
    }

    // If Windows has active render endpoints but no role assignment at this
    // instant, do not choose an arbitrary device. A sole endpoint is unambiguous.
    std::vector<Endpoint> endpoints;
    hr = EnumerateRenderEndpoints(endpoints);
    if (FAILED(hr)) return Fail(L"EnumAudioEndpoints fallback", hr, kExitEnumeration);
    if (endpoints.size() == 1) {
        std::wcout << L"DEFAULT_RESOLUTION\tsingle-active-fallback\n";
        return PrintEndpoint(L"DEFAULT", endpoints.front());
    }

    // This build is currently personalized for the Noire X / FiiO endpoint.
    // When several render devices are active and Windows temporarily exposes no
    // default role, accept that endpoint only if the match is unique.
    const std::wstring preferred[] = {L"Dan Clark Noire X", L"FiiO Q series", L"FiiO"};
    for (const auto& needle : preferred) {
        const Endpoint* match = nullptr;
        size_t matches = 0;
        for (const auto& candidate : endpoints) {
            if (ContainsInsensitive(candidate.name, needle)) {
                match = &candidate;
                ++matches;
            }
        }
        if (matches == 1 && match) {
            std::wcout << L"DEFAULT_RESOLUTION\tpreferred-active-fallback\t" << needle << L'\n';
            return PrintEndpoint(L"DEFAULT", *match);
        }
    }

    std::wcerr << L"ERROR\tDEFAULT_ENDPOINT_AMBIGUOUS\tACTIVE=" << endpoints.size()
               << L"\tLAST=" << HResultText(lastDefaultHr) << L'\n';
    for (const auto& endpoint : endpoints) PrintEndpoint(L"ACTIVE", endpoint);
    return kExitEnumeration;
}

int InstallDriverCommand(const std::wstring& infPath) {
    BOOL needReboot = FALSE;
    if (!DiInstallDriverW(nullptr, infPath.c_str(), 0, &needReboot)) {
        const DWORD error = GetLastError();
        std::wcerr << L"ERROR\tDiInstallDriverW\t" << Win32Text(error) << L'\n';
        return kExitDriver;
    }
    std::wcout << L"DRIVER_INSTALLED\tREBOOT=" << (needReboot ? 1 : 0) << L'\n';
    return 0;
}

} // namespace

int wmain(int argc, wchar_t** argv) {
    if (argc < 2) {
        std::wcerr << L"usage: OmniphonyEndpointCtl <probe-policy|list|find-name|set-default-name|set-default-id|get-default|install-driver> ...\n";
        return kExitUsage;
    }

    const std::wstring command = argv[1];

    if (command == L"install-driver") {
        if (argc != 3 || !argv[2] || !*argv[2]) {
            std::wcerr << L"ERROR\tinstall-driver requires one full INF path\n";
            return kExitUsage;
        }
        return InstallDriverCommand(argv[2]);
    }

    ComApartment com;
    if (FAILED(com.status())) return Fail(L"CoInitializeEx", com.status(), kExitCom);

    if (command == L"probe-policy") {
        ComPtr<IPolicyConfig> policy;
        const HRESULT hr = CreatePolicyConfig(policy);
        if (FAILED(hr)) return Fail(L"CPolicyConfigClient/IPolicyConfig", hr, kExitCom);
        std::wcout << L"POLICY_OK\tF8679F50-850A-41CF-9C72-430F290290C8\n";
        return 0;
    }

    if (command == L"list") {
        std::vector<Endpoint> endpoints;
        const HRESULT hr = EnumerateRenderEndpoints(endpoints);
        if (FAILED(hr)) return Fail(L"EnumAudioEndpoints", hr, kExitEnumeration);
        for (const auto& endpoint : endpoints) PrintEndpoint(L"ENDPOINT", endpoint);
        return 0;
    }

    if (command == L"get-default") return GetDefaultCommand();

    if (command == L"find-name" || command == L"set-default-name") {
        const auto needles = Needles(argc, argv, 2);
        if (needles.empty()) {
            std::wcerr << L"ERROR\tname match requires at least one non-empty needle\n";
            return kExitUsage;
        }
        Endpoint endpoint;
        const HRESULT findHr = FindByName(needles, endpoint);
        if (HRESULT_CODE(findHr) == ERROR_NOT_FOUND) return kExitNotFound;
        if (FAILED(findHr)) return Fail(L"find render endpoint", findHr, kExitEnumeration);
        if (command == L"find-name") return PrintEndpoint(L"ENDPOINT", endpoint);

        const HRESULT setHr = SetDefault(endpoint.id, true);
        if (FAILED(setHr)) return Fail(L"SetDefaultEndpoint/verify", setHr, kExitVerify);
        return PrintEndpoint(L"SET", endpoint);
    }

    if (command == L"set-default-id") {
        if (argc != 3 || !argv[2] || !*argv[2]) {
            std::wcerr << L"ERROR\tset-default-id requires one IMMDevice identifier\n";
            return kExitUsage;
        }
        const std::wstring id = argv[2];
        const HRESULT hr = SetDefault(id, true);
        if (FAILED(hr)) return Fail(L"SetDefaultEndpoint/verify", hr, kExitVerify);
        std::wcout << L"SET_ID\t" << id << L'\n';
        return 0;
    }

    std::wcerr << L"ERROR\tunknown command: " << command << L'\n';
    return kExitUsage;
}
