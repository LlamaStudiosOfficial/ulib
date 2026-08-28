using System;
using System.Collections.Concurrent;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;

namespace UlibRuntime
{
    /// <summary>
    /// Native FFI bindings into the Rust `ulib` library.
    /// </summary>
    internal static class Native
    {
        private const string Lib = "ulib";

        internal delegate void SignalCallback(
            IntPtr signalName,
            IntPtr userdata);

        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        internal static extern IntPtr ulib_window_create(uint width, uint height);

        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        internal static extern void ulib_window_set_title(IntPtr handle, [MarshalAs(UnmanagedType.LPStr)] string title);

        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        internal static extern void ulib_window_set_fullscreen(IntPtr handle, [MarshalAs(UnmanagedType.I1)] bool fullscreen);

        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        internal static extern void ulib_window_set_width(IntPtr handle, uint width);

        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        internal static extern void ulib_window_set_height(IntPtr handle, uint height);

        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        internal static extern uint ulib_window_get_width(IntPtr handle);

        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        internal static extern uint ulib_window_get_height(IntPtr handle);

        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int ulib_window_poll(IntPtr handle);

        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        internal static extern void ulib_window_close(IntPtr handle);

        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        internal static extern void ulib_window_destroy(IntPtr handle);

        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        internal static extern IntPtr ulib_module_load([MarshalAs(UnmanagedType.LPStr)] string path);

        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        internal static extern void ulib_module_free(IntPtr module);

        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        internal static extern void ulib_window_load_module(IntPtr handle, IntPtr module);

        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        internal static extern void ulib_window_set_signal_callback(
            IntPtr handle,
            SignalCallback cb,
            IntPtr userdata);
    }

    /// <summary>
    /// A native window backed by Rust/winit.
    /// </summary>
    public sealed class ULibWindow : IDisposable
    {
        private IntPtr _handle;
        private bool _disposed;

        internal ULibWindow(IntPtr handle, uint width, uint height)
        {
            _handle = handle;
            Width = width;
            Height = height;
        }

        public ULibWindow(uint width, uint height)
            : this(Native.ulib_window_create(width, height), width, height)
        {
            if (_handle == IntPtr.Zero)
                throw new InvalidOperationException("Failed to create window (native backend).");
        }

        public uint Width { get; }
        public uint Height { get; }

        private static uint GetWidth(IntPtr h) => Native.ulib_window_get_width(h);
        private static uint GetHeight(IntPtr h) => Native.ulib_window_get_height(h);

        public bool Fullscreen
        {
            set
            {
                EnsureNotDisposed();
                Native.ulib_window_set_fullscreen(_handle, value);
            }
        }

        public string Title
        {
            set
            {
                EnsureNotDisposed();
                Native.ulib_window_set_title(_handle, value);
            }
        }

        public void Close()
        {
            EnsureNotDisposed();
            Native.ulib_window_close(_handle);
        }

        /// <summary>
        /// Poll for events. Returns true if the window has been asked to close.
        /// </summary>
        public bool Poll() => Native.ulib_window_poll(_handle) != 0;

        private Thread _pollThread;
        private volatile bool _pollRunning;

        /// <summary>
        /// Starts a background thread that pumps events until the window closes.
        /// The process stays alive while the window is open.
        /// </summary>
        public void StartPoll()
        {
            EnsureNotDisposed();
            if (_pollThread != null)
                return;
            _pollRunning = true;
            _pollThread = new Thread(() =>
            {
                while (_pollRunning)
                {
                    if (Poll())
                    {
                        Close();
                        break;
                    }
                    Thread.Sleep(10);
                }
                _pollRunning = false;
            })
            {
                IsBackground = false,
                Name = "ulib-poll"
            };
            _pollThread.Start();
        }

        /// <summary>
        /// Stops the background polling thread.
        /// </summary>
        public void StopPoll()
        {
            _pollRunning = false;
            _pollThread?.Join(TimeSpan.FromMilliseconds(200));
            _pollThread = null;
        }

        /// <summary>
        /// Easy way to manage polling and threading automatically.
        /// Blocks until the window closes, then returns.
        /// </summary>
        public void Autostart()
        {
            StartPoll();
        }

        /// <summary>
        /// Easy way to stop the window: closes it and stops polling.
        /// </summary>
        public void Autostop()
        {
            Close();
            StopPoll();
        }

        /// <summary>
        /// Load a parsed .ulib module into this window, replacing its widget tree.
        /// </summary>
        public void LoadModule(UlibModule module)
        {
            EnsureNotDisposed();
            if (module == null || module.Handle == IntPtr.Zero)
                throw new ArgumentNullException(nameof(module));
            Native.ulib_window_load_module(_handle, module.Handle);
        }

        /// <summary>
        /// Register a handler for a button signal produced by this window's UI.
        /// The handler is invoked on the main thread via the signal queue.
        /// </summary>
        public void OnSignal(string signalName, Action handler)
        {
            EnsureNotDisposed();
            SignalRouter.Register(signalName, handler);
            EnsureSignalCallback();
        }

        private void EnsureSignalCallback()
        {
            if (_cbRegistered)
                return;
            _cbRegistered = true;
            _signalCallback = SignalRouter.NativeCallback;
            Native.ulib_window_set_signal_callback(_handle, _signalCallback, IntPtr.Zero);
        }

        private bool _cbRegistered;
        private Native.SignalCallback _signalCallback;

        private void EnsureNotDisposed()
        {
            if (_disposed || _handle == IntPtr.Zero)
                throw new ObjectDisposedException(nameof(ULibWindow));
        }

        public void Dispose()
        {
            if (_disposed)
                return;
            _disposed = true;
            if (_handle != IntPtr.Zero)
            {
                Native.ulib_window_destroy(_handle);
                _handle = IntPtr.Zero;
            }
        }

        ~ULibWindow() => Dispose();
    }

    /// <summary>
    /// A parsed .ulib module (a widget tree + optional stylesheet).
    /// </summary>
    public sealed class UlibModule : IDisposable
    {
        internal IntPtr Handle { get; private set; }

        internal UlibModule(IntPtr handle)
        {
            Handle = handle;
        }

        public void Dispose()
        {
            if (Handle != IntPtr.Zero)
            {
                Native.ulib_module_free(Handle);
                Handle = IntPtr.Zero;
            }
        }

        ~UlibModule() => Dispose();
    }

    /// <summary>
    /// Routes signal names from the native side to C# handlers, marshaling the
    /// callback onto the .NET thread pool so user code runs safely.
    /// </summary>
    internal static class SignalRouter
    {
        private static readonly ConcurrentDictionary<string, Action> Handlers =
            new ConcurrentDictionary<string, Action>(StringComparer.Ordinal);

        public static void Register(string name, Action handler)
        {
            Handlers[name] = handler;
        }

        /// <summary>
        /// The single native callback. Invoked on the native event-loop thread;
        /// it only enqueues the signal and lets the thread pool dispatch it.
        /// </summary>
        public static void NativeCallback(IntPtr signalName, IntPtr userdata)
        {
            string name = Marshal.PtrToStringUTF8(signalName);
            if (string.IsNullOrEmpty(name))
                return;

            if (Handlers.TryGetValue(name, out Action handler))
            {
                ThreadPool.QueueUserWorkItem(_ =>
                {
                    try
                    {
                        handler();
                    }
                    catch (Exception ex)
                    {
                        Console.Error.WriteLine($"[ULib] handler for '{name}' threw: {ex}");
                    }
                });
            }
        }
    }

    /// <summary>
    /// Static entry point for creating UI elements.
    /// </summary>
    public static class ULib
    {
        /// <summary>
        /// Create a new window backed by the native backend.
        /// </summary>
        public static ULibWindow Window(uint width, uint height) => new ULibWindow(width, height);

        /// <summary>
        /// Load a .ulib module from a file path. The module embeds the widget
        /// markup and any stylesheet referenced by a Style(...) directive.
        /// </summary>
        public static UlibModule LoadModule(string path)
        {
            if (string.IsNullOrEmpty(path))
                throw new ArgumentNullException(nameof(path));
            IntPtr handle = Native.ulib_module_load(path);
            if (handle == IntPtr.Zero)
                throw new InvalidOperationException($"Failed to load ulib module: {path}");
            return new UlibModule(handle);
        }

        /// <summary>
        /// Register a handler for a button signal by name.
        /// </summary>
        public static void OnSignal(string signalName, Action handler)
        {
            if (string.IsNullOrEmpty(signalName))
                throw new ArgumentNullException(nameof(signalName));
            SignalRouter.Register(signalName, handler);
        }
    }
}
