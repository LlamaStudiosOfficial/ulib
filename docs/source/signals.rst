Signals & events
================

Buttons in a ``.ulib`` module carry a signal identifier. When a button is
clicked in the native window, that signal name is routed back to C#.

Registering a handler
---------------------

.. code-block:: csharp

   ULib.OnSignal("submit", () => Console.WriteLine("Submit clicked"));

For a specific window instance:

.. code-block:: csharp

   ULibWindow win = new ULibWindow(400, 300);
   win.LoadModule(module);
   win.OnSignal("save", () => SaveFile());

How it works
------------

1. The native event loop performs a hit-test on mouse clicks against the
   placed widgets.
2. If a button is hit, its signal name is marshaled to C# through a single
   native callback.
3. The C# router (``SignalRouter``) looks up registered handlers by name and
   invokes them on the .NET thread pool, so you do not need to touch the
   native thread.

Both buttons in a module may share the same signal name, in which case the
same handler runs for either one.
