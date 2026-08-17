# Omniphony Windows Spatial Sound provider probe

This is a bounded Windows experiment for one question:

> Can an independently registered Omniphony format be enumerated by the Windows **Spatial sound** selector?

It is **not** a spatial renderer and it does not change Omniphony's audible signal path.

## Safety boundary

The probe is intentionally smaller than the product path:

- it writes only two project-owned registry subtrees;
- it does not write `MMDevices` state;
- it does not change the default playback endpoint;
- it does not install a virtual audio device;
- it does not restart Windows Audio;
- it does not hook or inject into applications;
- it does not replace Windows system files or HRTFs;
- the COM DLL implements only `IUnknown`, so unsupported renderer interfaces fail cleanly with `E_NOINTERFACE`;
- `unregister` deletes only the two Omniphony probe keys.

Stable experimental identities:

```text
Spatial format GUID  {4BD75423-A66C-4586-B782-1FCBBDF2AE74}
COM provider CLSID   {F3CDF827-20C4-405E-A430-8F739343FC89}
```

Candidate registration surface under test:

```text
HKLM\SOFTWARE\Microsoft\Multimedia\Audio\Spatial\Encoder\{format-guid}
HKLM\SOFTWARE\Classes\CLSID\{provider-clsid}\InProcServer32
```

The first path is an experimentally inferred Windows registration seam, not a documented public provider contract. A successful registry write is therefore not the result. Windows Settings must independently enumerate the format.

## Run the experiment

Extract `OmniphonySpatialProbeCtl.exe` and `OmniphonySpatialProbe.dll` into the same directory and leave them there while the probe is registered.

First capture the current state from a normal terminal:

```powershell
.\OmniphonySpatialProbeCtl.exe contract
.\OmniphonySpatialProbeCtl.exe list
.\OmniphonySpatialProbeCtl.exe status
```

`status` returns exit code 3 before registration by design.

Then open **PowerShell as Administrator** in that directory and run:

```powershell
.\OmniphonySpatialProbeCtl.exe register .\OmniphonySpatialProbe.dll
.\OmniphonySpatialProbeCtl.exe diagnose
```

The registration command verifies that the inert COM DLL can be activated through the newly written CLSID before it leaves the registry state in place.

Now close and reopen Windows Settings and inspect:

```text
Settings
→ System
→ Sound
→ Dan Clark Noire X / current physical output
→ Spatial sound
```

### Result A: `Omniphony` appears

This proves the **enumeration seam**. It does not yet prove that Windows will accept Omniphony as a functional spatial renderer. Do not interpret selection failure as failure of enumeration.

Record the dropdown result, then run `diagnose` again. The next experiment can replace the inert COM object with the smallest real interface Windows requests.

### Result B: `Omniphony` does not appear

This falsifies the current registration hypothesis on that Windows build. Preserve the `list`, `status`, and `diagnose` output. The next step is Process Monitor / registry-delta observation around a known provider, not progressively broader registry writes.

## Clean removal

From an elevated terminal:

```powershell
.\OmniphonySpatialProbeCtl.exe unregister
.\OmniphonySpatialProbeCtl.exe status
```

After `unregister`, `status` should again return exit code 3 and report both owned keys absent.

## Evidence states

Keep these claims separate:

```text
build succeeds
≠ COM activation succeeds
≠ Windows Settings enumerates Omniphony
≠ Windows accepts Omniphony as a selectable renderer
≠ spatial audio is rendered correctly
```

This probe is complete when the first uncertain boundary, **Windows Settings enumeration**, has a real-machine result.
