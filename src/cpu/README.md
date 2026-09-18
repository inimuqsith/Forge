# CPU Profiler Module (`src/cpu/`)

Mengimplementasikan fungsionalitas `forge cpu-dump`:
- Introspeksi CPUID, `/proc/cpuinfo`, dan compiler feature introspection.
- Deteksi microarchitecture (misal: `znver4`, `znver3`, `alderlake`, `x86-64-v4`).
- Deteksi ekstensi ISA (AVX-512, AVX2, FMA, VAES, SHA-NI, SSE4.2, dll.).
- Penghitungan CFLAGS/CXXFLAGS optimal untuk CPU host.
- Generasi dan export berkas `cpu-profile.json`.
