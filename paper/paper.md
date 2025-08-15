---
title: 'showmaker: A Simple Tool for Rendering Quarto Presentations from Markdown'
tags:
  - AI
  - Tauri
  - Quarto
  - Markdown
  - Presentation
authors:
  - name: Jinhwan Kim
    orcid: 0009-0009-3217-2417
    affiliation: 1
affiliations:
  - name: jahnen, KOREA
    index: 1
date: 15 August 2025
bibliography: paper.bib
---

# Summary

`showmaker` is an open-source, standalone desktop application that
transforms Markdown documents into interactive HTML slides using Quarto.

Built with Rust and Tauri, it eliminates the need for command-line
setup, offering a simple graphical interface for preparing, rendering,
and exporting presentations.  

The software supports simple and essential features 
making it suitable for educators, researchers, and non-technical presenters.

# Statement of Need

Creating web-based presentations from Markdown often requires manual
installation of Quarto, correct YAML configuration, and command-line
usage, which can be a barrier for non-technical users.

Although there are AI-powered services that help generate slide content,
these typically do not produce ready-to-use interactive slides directly.
Instead, they often export only raw text or static formats, requiring
additional manual work to convert into presentation-ready HTML.  

In many cases, such services are also subscription-based, introducing
recurring costs that can be a burden for educators, students, or
non-profit researchers.

`showmaker` addresses this gap by providing a no-CLI, one-click, free
workflow for generating Quarto slides from Markdown — including content
that may have been created with AI tools — at zero cost and with full
offline capability, ensuring both speed and data privacy.

# State of the Field

Several existing tools (e.g., Quarto CLI, RStudio’s presentation
support, and web-based editors) allow creating HTML slides, but they
either require a programming environment, lack offline capabilities, or
are limited to specific platforms.  
`showmaker` differentiates itself by:
- Being cross-platform (Windows/macOS) with an installer
- Validating input before rendering to reduce errors
- Integrating AI-assisted Markdown generation
- Operating fully offline for privacy-sensitive workflows

# Functionality

Key features include:
- Upload Markdown files
- Render to HTML with a single click
- Use AI prompt templates for rapid slide generation
- Operate fully offline without external dependencies after installation
- Validate YAML front matter (title, author, format, embed-resources)

# Acknowledgements

The author thanks the open-source communities behind Tauri,
and Quarto for providing the foundation for this work.

# References
