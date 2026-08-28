Styling (CSS subset)
====================

ULib supports a small CSS subset for styling widgets. Stylesheets are loaded
from a ``.ulib`` module via the ``Style("file.css")`` directive.

Selectors
---------

* ``window`` — the window / root container
* ``label`` — all labels
* ``button`` — all buttons
* ``hbox`` — all horizontal containers
* ``vbox`` — all vertical containers
* ``*`` — everything (base styles)

Properties
----------

.. list-table::
   :widths: 25 75
   :header-rows: 1

   * - Property
     - Description
   * - ``background``
     - Background color, ``#rrggbb``.
   * - ``color``
     - Text color, ``#rrggbb``.
   * - ``border-color``
     - Border color, ``#rrggbb``.
   * - ``border-size``
     - Border thickness in px.
   * - ``padding``
     - Inner padding in px.
   * - ``margin``
     - Outer margin in px.
   * - ``align``
     - ``left``, ``center`` or ``right``.

Example
-------

.. code-block:: css

   window {
       background: #222222;
       color: #ffffff;
       border-size: 0;
       padding: 8;
   }

   button {
       background: #2266aa;
       color: #ffffff;
       border-color: #88bbff;
       border-size: 2;
       padding: 4;
       margin: 2;
   }

   label {
       color: #ffd34d;
       align: center;
   }
