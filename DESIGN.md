---
version: alpha
name: Kaku
description: Design system for kaku.fun, a quiet macOS terminal product with an AI-coding focus, serif-forward editorial hierarchy, and minimal blue-accented controls.
colors:
  primary: "#1b365d"
  secondary: "#faf9f5"
  tertiary: "#e5e7eb"
  neutral: "#141413"
  surface: "#f5f4ed"
  on-surface: "#141413"
  background: "#f5f4ed"
  error: "#b42318"
typography:
  headline-display:
    fontFamily: "Charter, Georgia, Palatino, Times New Roman, serif"
    fontSize: 85.05px
    fontWeight: 500
    lineHeight: 86.751px
    letterSpacing: "-1.4px"
  headline-lg:
    fontFamily: "Charter, Georgia, Palatino, Times New Roman, serif"
    fontSize: 37.8px
    fontWeight: 500
    lineHeight: 44.604px
    letterSpacing: 0px
  headline-md:
    fontFamily: "Charter, Georgia, Palatino, Times New Roman, serif"
    fontSize: 20px
    fontWeight: 500
    lineHeight: 24px
    letterSpacing: 0px
  body-lg:
    fontFamily: "-apple-system, BlinkMacSystemFont, SF Pro Text, Helvetica Neue, Arial, sans-serif"
    fontSize: 16px
    fontWeight: 500
    lineHeight: 24px
    letterSpacing: 0.4px
  body-md:
    fontFamily: "-apple-system, BlinkMacSystemFont, SF Pro Text, Helvetica Neue, Arial, sans-serif"
    fontSize: 16px
    fontWeight: 500
    lineHeight: 24px
    letterSpacing: 0.4px
  body-sm:
    fontFamily: "-apple-system, BlinkMacSystemFont, SF Pro Text, Helvetica Neue, Arial, sans-serif"
    fontSize: 15px
    fontWeight: 400
    lineHeight: 22px
    letterSpacing: 0.2px
  label-lg:
    fontFamily: "Charter, Georgia, Palatino, Times New Roman, serif"
    fontSize: 15px
    fontWeight: 500
    lineHeight: 1.2
    letterSpacing: 0px
  label-md:
    fontFamily: "Charter, Georgia, Palatino, Times New Roman, serif"
    fontSize: 15px
    fontWeight: 500
    lineHeight: 1.2
    letterSpacing: 0px
  label-sm:
    fontFamily: "Charter, Georgia, Palatino, Times New Roman, serif"
    fontSize: 15px
    fontWeight: 400
    lineHeight: 1.2
    letterSpacing: 0px
rounded:
  none: 0px
  sm: 8px
  md: 999px
  lg: 999px
  xl: 999px
  full: 999px
spacing:
  xs: 2px
  sm: 10px
  md: 18px
  lg: 28px
  xl: 64px
components:
  button:
    primary:
      backgroundColor: "#1b365d"
      color: "#faf9f5"
      borderColor: "#1b365d"
      borderRadius: 999px
      borderWidth: 1px
      borderStyle: solid
      padding: "12px 26px"
      fontSize: 15px
      fontWeight: 500
      minWidth: 155px
      minHeight: 44px
      textDecoration: none
      boxShadow: none
      fontFamily: "Charter, Georgia, Palatino, Times New Roman, serif"
    secondary:
      backgroundColor: transparent
      color: "#1b365d"
      borderColor: "#1b365d"
      borderRadius: 999px
      borderWidth: 1px
      borderStyle: solid
      padding: "12px 26px"
      fontSize: 15px
      fontWeight: 500
      minWidth: 155px
      minHeight: 44px
      textDecoration: none
      boxShadow: none
      fontFamily: "Charter, Georgia, Palatino, Times New Roman, serif"
    link:
      backgroundColor: transparent
      color: "#1b365d"
      borderColor: transparent
      borderRadius: 0px
      borderWidth: 0px
      borderStyle: none
      padding: 0px
      fontSize: 15px
      fontWeight: 400
      minWidth: 0px
      minHeight: 0px
      textDecoration: none
      boxShadow: none
      fontFamily: "Charter, Georgia, Palatino, Times New Roman, serif"
  card:
    backgroundColor: "#f5f4ed"
    borderColor: "#e5e7eb"
    borderRadius: 8px
    borderWidth: 1px
    borderStyle: solid
    padding: 16px
    boxShadow: none
    textColor: "#141413"
---
# Overview

Kaku is a light, editorial, macOS-first product site for an AI-era terminal. The visual language is restrained: warm off-white surfaces, deep navy accents, large serif headlines, and minimal chrome. The screenshot and homepage copy show a product that should feel calm, trustworthy, and technical without appearing heavy.

Use this document as the source of truth for site UI, marketing pages, and documentation surfaces. Prefer clarity, whitespace, and strong hierarchy over decoration.

# Colors

## Core palette

- **Primary**: `#1b365d` — brand navy used for key actions, links, and top-level emphasis.
- **Secondary**: `#faf9f5` — button text and light contrast against primary fills.
- **Tertiary**: `#e5e7eb` — subtle borders and separators.
- **Neutral**: `#141413` — primary text color.
- **Surface / Background**: `#f5f4ed` — the dominant canvas color.
- **Error**: `#b42318` — reserved for destructive states and command failures.

## Usage guidance

The interface should stay mostly neutral. Use navy sparingly for:
- primary CTA buttons
- active tabs and links
- selected pills or small badges
- code-path affordances

Avoid introducing bright UI colors. Error red should appear only where the product communicates failure, command issues, or destructive risk.

# Typography

## Type system

- **Headline display**: large serif hero titles.
- **Headline lg**: section titles such as “Real product surfaces, not concept art.”
- **Headline md**: subheads and feature titles.
- **Body lg / body md**: explanatory copy and section descriptions.
- **Body sm**: supporting labels, stats, and minor helper text.
- **Label sizes**: button text, nav labels, and compact UI controls.

## Behavior

The design uses a serif family for headlines and a system sans for body text, matching the homepage hero and feature sections.

### Recommended usage
- Use serif headlines for branding, section anchors, and feature-led messaging.
- Use system body text for paragraphs, stats, and operational UI.
- Keep letter spacing tight or neutral; only the hero headline should use negative tracking.
- Maintain generous line height for editorial copy.

### Code and terminal content
Terminal text, command snippets, and shell examples may use the product’s native terminal font stack if available, but the marketing site should remain consistent with the serif/sans split above.

# Layout

The page is centered around a single-column hero with a restrained top nav and wide whitespace margins.

## Structure
- Top navigation: brand left, utility links and language toggle right.
- Hero block: label, display title, short description, two CTAs, stats row, and pill tags.
- Feature sections: section index label, large headline, short supporting line, then two-column screenshot grids or feature cards.
- Content width should be narrow enough to keep typography readable and centered.

## Spacing
Use the following spacing tokens for vertical rhythm:
- `xs`: micro gaps between iconography and text
- `sm`: compact control spacing and pill padding
- `md`: standard list and stat spacing
- `lg`: section separation
- `xl`: page-scale breaks between major blocks

The screenshot suggests very large whitespace above the hero and between major sections. Preserve that airiness.

# Elevation & Depth

This system is intentionally flat.

- Shadows are effectively none across the product UI.
- Avoid layered cards, heavy glows, and floating surfaces.
- Use borders, contrast, and whitespace to separate regions instead of depth.
- If a component needs emphasis, prefer color and outline treatment over drop shadows.

# Shapes

The shape language is soft and simple.

- Buttons use fully rounded pills.
- Tags and status chips use pill radii with thin borders.
- Cards use a subtle 8px radius.
- The overall product should feel rounded but not playful.

Recommended rounded tokens:
- `none`: square edges for structural elements
- `sm`: cards and panels
- `md` through `full`: pill controls and compact chips

# Components

## Buttons

### Primary button
Use for the single strongest action on a page, usually the download or install CTA.
- Fill: navy
- Text: off-white
- Radius: pill
- Min height: 44px
- Min width: 155px

### Secondary button
Use for alternate high-intent actions such as GitHub or docs.
- Transparent fill
- Navy border and text
- Same dimensions as primary

### Link button
Use for low-emphasis navigation and inline actions.
- No border
- No fill
- Navy text only
- No shadow

## Cards

Use cards for screenshots, feature tiles, and compact info blocks.
- Keep the background aligned to the page surface.
- Use a thin neutral border.
- Avoid shadow-based separation.
- Keep internal padding moderate; do not overfill the surface.

## Pills and tags

Small pill tags are used for product claims such as “Built on WezTerm” and “Zero-config start.”
- Keep them compact and low-contrast.
- Use neutral borders and surface fills.
- Typography should be small and readable, not decorative.

## Navigation

Top nav items should be minimal, text-led, and right-aligned where possible.
- Brand name on the left
- Light utility controls on the right
- Prefer simple text or pill buttons over icon-heavy navigation

## Feature sections

Feature sections should follow this structure:
1. compact index label
2. large serif headline
3. short explanatory sentence
4. content grid or screenshot pair

Keep the visual rhythm consistent across sections and avoid introducing new section styles.

# Do's and Don'ts

## Do
- Use `#f5f4ed` as the default canvas for most pages.
- Use serif headlines for the marketing narrative and system sans for supporting copy.
- Keep primary actions in navy pill buttons with strong contrast.
- Preserve large whitespace around the hero and between major sections.
- Use thin borders and subtle radius instead of shadows.
- Keep screenshots framed simply and let the product UI provide visual richness.
- Make command examples readable and realistic; they should feel like actual terminal interactions.
- Keep copy calm, specific, and product-led.

## Don't
- Don’t introduce gradients, neon colors, or glassmorphism.
- Don’t use heavy shadows or stacked card elevations.
- Don’t make buttons square, oversized, or visually noisy.
- Don’t rely on color alone to communicate command failures; pair error states with text.
- Don’t crowd the hero with extra CTAs, badges, or promotional clutter.
- Don’t replace the serif headline style with a geometric sans for marketing pages.
- Don’t add decorative borders, icon badges, or motion effects that compete with the product content.
- Don’t change the palette to feel more “app store” or more “startup neon”; keep it quiet and macOS-native.
