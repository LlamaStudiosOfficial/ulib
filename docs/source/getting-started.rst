Getting started
===============

Requirements
------------

* Rust (stable) — used to build the native library
* .NET SDK 8 or later — used by the C# side
* A Desktop session with an X11 or Wayland display

Build the native library
------------------------

Building the C# project triggers a Rust release build automatically (via a
``BuildRustLib`` MSBuild target), but you can also build it by hand::

    cargo build --release

This produces ``target/release/libulib.so`` (Linux), ``libulib.dylib`` (macOS)
or ``ulib.dll`` (Windows). The MSBuild target copies it next to your .NET
executable automatically.

Add the C# library to your project
----------------------------------

.. code-block:: bash

   dotnet add reference /path/to/ulib/csharp/ULib.csproj

Then import the namespace:

.. code-block:: csharp

   using UlibRuntime;

Create a window and show it
---------------------------

.. code-block:: csharp

   using UlibRuntime;

   ULibWindow win = new ULibWindow(700, 500);
   win.Title = "Hello";
   win.Autostart();

   // ... later ...
   win.Autostop();
