#include <windows.h>
#include <audioenginebaseapo.h>
#include <iostream>
#include <wrl/client.h>

using Microsoft::WRL::ComPtr;

namespace {
constexpr GUID kOmniphonyApoClsid = {
    0xa9333bfe, 0x39c1, 0x40fd, {0xb4, 0xb0, 0xec, 0xc5, 0x91, 0x41, 0x0b, 0x47}};
}

int wmain() {
    const HRESULT init = CoInitializeEx(nullptr, COINIT_MULTITHREADED);
    if (FAILED(init)) {
        std::wcerr << L"COM_INIT_FAILED\t0x" << std::hex << init << L"\n";
        return 2;
    }

    ComPtr<IAudioProcessingObject> apo;
    HRESULT hr = CoCreateInstance(kOmniphonyApoClsid, nullptr, CLSCTX_INPROC_SERVER,
                                  IID_PPV_ARGS(apo.ReleaseAndGetAddressOf()));
    if (FAILED(hr)) {
        std::wcerr << L"APO_ACTIVATION_FAILED\t0x" << std::hex << hr << L"\n";
        CoUninitialize();
        return 3;
    }

    ComPtr<IAudioProcessingObjectRT> rt;
    ComPtr<IAudioProcessingObjectConfiguration> configuration;
    ComPtr<IAudioSystemEffects> effects;
    const HRESULT rtHr = apo.As(&rt);
    const HRESULT cfgHr = apo.As(&configuration);
    const HRESULT fxHr = apo.As(&effects);
    if (FAILED(rtHr) || FAILED(cfgHr) || FAILED(fxHr)) {
        std::wcerr << L"APO_INTERFACE_FAILED\tRT=0x" << std::hex << rtHr
                   << L"\tCFG=0x" << cfgHr << L"\tFX=0x" << fxHr << L"\n";
        CoUninitialize();
        return 4;
    }

    HNSTIME latency = -1;
    hr = apo->GetLatency(&latency);
    if (FAILED(hr) || latency != 0) {
        std::wcerr << L"APO_LATENCY_FAILED\tHR=0x" << std::hex << hr
                   << L"\tLATENCY=" << std::dec << latency << L"\n";
        CoUninitialize();
        return 5;
    }

    std::wcout << L"APO_COM_OK\tCLSID={A9333BFE-39C1-40FD-B4B0-ECC591410B47}\tLATENCY_HNS=0\n";
    CoUninitialize();
    return 0;
}
