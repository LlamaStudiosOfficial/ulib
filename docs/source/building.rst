Building & testing
==================

The repository layout
---------------------

.. code-block:: text

   ulib/
   ├── Cargo.toml            # Rust crate (cdylib + lib)
   ├── src/
   │   ├── lib.rs            # C ABI / FFI exports
   │   ├── window.rs         # winit event loop + rendering
   │   ├── ui.rs             # .ulib parser, layout, render, hit-test
   │   ├── style.rs          # CSS subset parser
   │   └── font.rs            # embedded 5x7 bitmap font
   └── csharp/
       ├── ULib.cs           # P/Invoke bindings + wrappers
       └── ULib.csproj       # .NET class library

Building
--------

Rust (produces the native shared library)::

    cargo build --release

Tests::

    cargo test

C# (also triggers the Rust release build)::

    dotnet build

Running
-------

Copy ``libulib.so`` (or equivalent) next to your executables — the ``ULib.csproj``
copies it to the output automatically. Then run your C# program with your
``.ulib`` and ``.css`` files alongside it.
