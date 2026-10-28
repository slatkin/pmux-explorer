## ADDED Requirements

### Requirement: the top row carries a side-switch control

The sidebar's top row SHALL carry a side-switch control next to the pop-out control. Clicking it, and pressing the tmux prefix followed by `Ctrl+O` while the sidebar has focus, SHALL move the docked sidebar to the opposite edge of the window.

#### Scenario: the control sits next to the pop-out control
- **WHEN** the sidebar is rendered
- **THEN** the top row shows the side-switch control directly left of the pop-out control

#### Scenario: the control moves a left sidebar to the right
- **WHEN** the sidebar is docked on the left and the user clicks the control
- **THEN** the sidebar is a full-height tiled pane at the right edge and the workspace panes keep their order

#### Scenario: the control moves a right sidebar to the left
- **WHEN** the sidebar is docked on the right and the user presses the tmux prefix followed by `Ctrl+O`
- **THEN** the sidebar is a full-height tiled pane at the left edge

#### Scenario: the shortcut outside the sidebar
- **WHEN** the user presses the tmux prefix followed by `Ctrl+O` outside the sidebar
- **THEN** tmux rotates the window as it does by default

### Requirement: the side switch applies to every window

The side switch SHALL move the sidebar of every window to the new side, and SHALL set the global `@sidebar_position` to that side so sidebars created later follow. It SHALL NOT change the user's tmux config.

#### Scenario: all windows follow
- **WHEN** a session has three windows with docked left sidebars and the user switches the side
- **THEN** all three sidebars are docked at the right edge

#### Scenario: a new window follows the switch
- **WHEN** the user switches the side to right and then opens a new window
- **THEN** the new window's sidebar is created on the right

### Requirement: windows that cannot be moved are skipped

The side switch SHALL skip a window that tmux refuses to rearrange, such as a zoomed window, and a window whose sidebar is floating. A skipped window SHALL keep its side. The switch SHALL still move the other windows.

#### Scenario: a zoomed window
- **WHEN** one of four windows is zoomed and the user switches the side
- **THEN** the other three sidebars move and the zoomed window keeps its sidebar on the old side

#### Scenario: a floating sidebar in another window
- **WHEN** the sidebar of one window is floating and the user switches the side from another window
- **THEN** the floating sidebar is left as it is and the docked sidebars move

### Requirement: the side switch reports its outcome

After a switch that moved or skipped at least one window, the sidebar SHALL tell the user how many windows moved and how many were skipped.

#### Scenario: some windows skipped
- **WHEN** the switch moves three windows and skips one
- **THEN** the sidebar shows a notice that three windows moved and one was skipped

#### Scenario: every window moved
- **WHEN** the switch moves all windows and skips none
- **THEN** the sidebar shows a notice that the windows moved, with no skip count

### Requirement: the side switch is unavailable while floating

While the sidebar is floating, the side-switch control SHALL be drawn disabled and SHALL NOT act. The floating panel SHALL keep its right-edge anchor.

#### Scenario: clicking the disabled control
- **WHEN** the sidebar is floating and the user clicks the side-switch control
- **THEN** nothing moves and the global side does not change

### Requirement: the rule draws the tabs as centered icons

The bottom panel's rule SHALL draw each shown tab as a single Nerd Font glyph — Tools , Tasks , Git 󰊢, Thought 󰟶 — instead of a text label, and SHALL center the tab run on the rule. Clicking a drawn glyph SHALL select its tab, and the click hitboxes SHALL be the drawn columns.

#### Scenario: a glyph per tab
- **WHEN** the bottom panel's rule renders
- **THEN** each shown tab draws its glyph and no tab draws a text label

#### Scenario: the run centers on the rule
- **WHEN** the rule is `W` columns wide and the tab run is `R` columns
- **THEN** the run starts at column `floor((W - R) / 2)` and the rule still spans exactly `W` columns

#### Scenario: a click on a glyph selects its tab
- **WHEN** the user clicks a drawn glyph
- **THEN** the rule selects that tab
