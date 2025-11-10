SylvScreenShare is a WIP Rust program to capture video game footage. 

This works by injecting into the program, and hooking the present function.

Currently the following APIs are implemented:
| API |  Status  |Reason|
|-----|----------|------|
|DX12 |✅ Working|N/A
|DX11 |✅ Working|N/A
|DX10 |✅ Working|N/A
|DX9  |❌ Broken |Needs CPU Buffer
|DX9ex|❌ Broken |Unimplemented
|DX8  |N/A       |Unplanned
|VK   |✅ Working|N/A
|OGL  |🚧 WIP    |Needs projection matrix

Support for savings videos is WIP, as there are not currently hardware encoder bindings that support weak linkage

The control flow is as follows, barring broken APIs
```mermaid
flowchart  TD
A[Startup] -->B(Injection)
B  -->|DX11+, OpenGL, Vulkan|C(Create NT Handle)
B  -->|DX9Ex, DX10| D[Create  DX  Handle]
B  -->|DX9-| E(WIP CPU Buffer)
C  -->CopyDst(Copy Into Shared Memory)
D  -->CopyDst(Copy Into Shared Memory)
E  -->|Map  Buffer|CopyDst(Copy Into Shared Memory)

A  --> G(Is Running)
G  -->|NT  Handle != Null| CopySrc(Copy from shared memory)
G  -->|Handle != Null| CopySrc
CopySrc  --> G
G  -->|Close|N(Null and Free resources)
N  --> Exit
```
