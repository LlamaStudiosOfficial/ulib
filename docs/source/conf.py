# -- Project information -----------------------------------------------------

project = "ULib"
copyright = "2026, ULib contributors"
author = "ULib contributors"
release = "0.1.0"

# -- General configuration ---------------------------------------------------

# The extension "sphinx.ext.autodoc" documents Rust types only loosely; we keep
# the docs hand-written. myst_parser (Markdown) is optional; the sources are RST,
# which Sphinx supports out of the box.
extensions = []

templates_path = ["_templates"]
exclude_patterns = []

# -- Options for HTML output -------------------------------------------------

html_theme = "sphinx_rtd_theme"
html_static_path = ["_static"]
