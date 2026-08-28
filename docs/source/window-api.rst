Window API
==========

``ULibWindow`` wraps a native window backed by winit and rendered with
softbuffer.

Creating a window
-----------------

.. code-block:: csharp

   ULibWindow win = new ULibWindow(800, 600);

The constructor creates the native window immediately. The process stays
alive on its own event-loop thread; the window is shown as soon as a UI
module is loaded.

Properties
----------

.. list-table::
   :widths: 30 70
   :header-rows: 1

   * - Member
     - Description
   * - ``Title`` (set)
     - Sets the window title bar text.
   * - ``Fullscreen`` (set)
     - Toggles fullscreen (borderless).
   * - ``Width`` (get)
     - The width requested at creation.
   * - ``Height`` (get)
     - The height requested at creation.

Methods
-------

.. list-table::
   :widths: 30 70
   :header-rows: 1

   * - Method
     - Description
   * - ``LoadModule(UlibModule)``
     - Replaces the window's widget tree with the parsed module.
   * - ``OnSignal(string, Action)``
     - Registers a handler for a button signal.
   * - ``Poll()``
     - Returns true when the window has been asked to close.
   * - ``Close()``
     - Requests the window to close.
   * - ``StartPoll()``
     - Starts a background thread that pumps events until close.
   * - ``StopPoll()``
     - Stops the background polling thread.
   * - ``Autostart()``
     - Convenience wrapper; manages polling and threading for you.
   * - ``Autostop()``
     - Closes the window and stops polling.
   * - ``Dispose()``
     - Frees the native window resources.

Beginner-friendly lifecycle
---------------------------

.. code-block:: csharp

   ULibWindow win = new ULibWindow(800, 600);
   win.Title = "Super Wow Window!";
   win.Autostart();   // manages poll + threads
   win.Autostop();    // closes and cleans up
