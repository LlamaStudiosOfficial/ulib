ULib
====

ULib is a small cross-platform GUI library written in **Rust**, exposed to
**C#** through a C ABI. It lets you build a window, lay out widgets with a
declarative markup file, and style them with a CSS subset.

At a glance
-----------

.. code-block:: csharp

   using System;
   using System.Threading;
   using UlibRuntime;

   // Load the UI module (widget markup + stylesheet).
   UlibModule module = ULib.LoadModule("app.ulib");

   ULibWindow win = new ULibWindow(1280, 720);
   win.LoadModule(module);
   win.Title = "My App";

   // Handle button clicks.
   ULib.OnSignal("submit", () => Console.WriteLine("Clicked!"));

   win.Autostart();
   win.Autostop();

.. toctree::
   :maxdepth: 2
   :caption: Contents

   getting-started
   window-api
   module-format
   styling
   signals
   examples
   building
