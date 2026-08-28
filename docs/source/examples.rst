Examples
========

Full application
----------------

**app.ulib**

.. code-block:: text

   Style(theme.css)

   Label("Transaction app")

   <VBOX>
   Label("Name")
   Button("Submit",submit)
   Button("Cancel",cancel)
   </VBOX>

**theme.css**

.. code-block:: css

   window  { background: #1b1b2f; color: #ffffff; padding: 10; }
   label   { color: #a0c4ff; align: left; }
   button  { background: #4a3f8f; color: #ffffff;
             border-color: #9d8fe0; border-size: 2; padding: 6; margin: 2; }

**Program.cs**

.. code-block:: csharp

   using System;
   using UlibRuntime;

   UlibModule module = ULib.LoadModule("app.ulib");
   ULibWindow win = new ULibWindow(640, 480);
   win.LoadModule(module);
   win.Title = "Transactions";

   ULib.OnSignal("submit", () => Console.WriteLine("Submitted"));
   ULib.OnSignal("cancel", () =>
   {
       Console.WriteLine("Cancelled");
       win.Autostop();
   });

   win.Autostart();
