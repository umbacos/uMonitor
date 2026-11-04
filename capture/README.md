# What we found in C:\Program Files\35inchENG\UsbMonitor.exe

UsbMonitor.exe is a program for Windows, built for x86_64 (64-bit). It looks like it was made with .NET (C#, F# or VB). It has no native symbol table, but its function names are kept in .NET metadata. This is a .NET assembly: use the .NET tools for readable C#-level results.

Evidence ev_a5c46d8cb484fef7c9dd2a0444d75a68b63e99697a28b58b1a262c74d6174b8d.

```json
{
  "architecture": "x86_64",
  "bits": 64,
  "dotnet_arch": "x64",
  "entry_point": null,
  "format": "pe",
  "format_name": "PE (Windows)",
  "kind": "executable",
  "libraries": [],
  "next_steps": [
    {
      "tool": "managed_assembly_show",
      "why": ".NET code decompiles far better as .NET than as machine code"
    },
    {
      "tool": "core_overview_show",
      "why": "a one-page summary: sections, imports, counts"
    },
    {
      "tool": "native_functions_list",
      "why": "every function we found"
    },
    {
      "tool": "native_strings_find",
      "why": "find text the program shows or uses"
    },
    {
      "tool": "native_function_study",
      "why": "a full dossier on one function (start with main or the entry point)"
    },
    {
      "tool": "core_feature_find",
      "why": "follow a word you see in the app to the code behind it"
    },
    {
      "tool": "native_pe_resources_list",
      "why": "icons, version information and the manifest"
    }
  ],
  "operating_system": "Windows",
  "plain_english": "UsbMonitor.exe is a program for Windows, built for x86_64 (64-bit). It looks like it was made with .NET (C#, F# or VB). It has no native symbol table, but its function names are kept in .NET metadata. This is a .NET assembly: use the .NET tools for readable C#-level results.",
  "recommended_toolsets": [
    "core",
    "native",
    "managed",
    "formats"
  ],
  "stripped": false,
  "symbol_table_stripped": true,
  "toolchain_hints": [
    {
      "category": "runtime",
      "evidence": "CLR runtime header",
      "id": "dotnet",
      "label": ".NET (C#, F# or VB)",
      "strength": "strong"
    }
  ]
}
```

## Ghidra

Deep mode: Ghidra has analysed C:\Program Files\35inchENG\UsbMonitor.exe (0 min), so decompiling it is quick.
