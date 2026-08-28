Module format (``.ulib``)
=========================

A ``.ulib`` file is a small declarative description of a window's widgets and
layout. It is parsed on the Rust side at load time.

A complete example
------------------

.. code-block:: text

   Style(style.css)

   Label("Wow text!")

   <HBOX>
   Button("Super button!",back)
   </HBOX>

   <VBOX>
   Button("Save",save)
   </VBOX>

Directives
----------

.. list-table::
   :widths: 20 80
   :header-rows: 1

   * - Directive
     - Description
   * - ``Style("file.css")``
     - Loads a stylesheet, resolved relative to the module file.
   * - ``Label("text")``
     - A static text label.
   * - ``Button("text", signal)``
     - A clickable button. ``signal`` is the identifier fired on click.
   * - ``<HBOX> ... </HBOX>``
     - A horizontal container; children share its width equally.
   * - ``<VBOX> ... </VBOX>``
     - A vertical container; children share its height equally.

The file already begins with an implicit vertical container, so top-level
widgets stack vertically.

Example module
''''''''''''''

.. code-block:: text

   Style(theme.css)

   Label("Welcome")

   <VBOX>
   Button("Open",open)
   Button("Quit",quit)
   </VBOX>
