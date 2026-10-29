# Spec Delta

## MODIFIED Requirements

### Requirement: the bottom panel is a split part under the agent list

In a column, the bottom panel SHALL be directly under the agent list, with
no blank row between them. One horizontal rule SHALL be the top row of the
bottom panel and SHALL separate the two parts. The bottom panel SHALL have
no side border and no bottom border, so its content uses the full width of
the sidebar. A strip SHALL NOT draw this rule; its tab panel is beside the
agent panel, behind the strip divider.

#### Scenario: the panel has only the rule
- **WHEN** the sidebar is a column and shows the bottom panel
- **THEN** the row directly above the panel content is a solid band across the full sidebar width, in the filter bar's background colour (`@sidebar_color_filter_bar_bg`)
- **AND** no border character is drawn at the left edge, the right edge, or the bottom row of the panel

#### Scenario: the content uses the full width
- **WHEN** the sidebar is a column 40 columns wide and shows the Activity tab
- **THEN** each activity line has 40 columns available

#### Scenario: a strip has no rule
- **WHEN** the sidebar is a strip and shows the tab panel
- **THEN** no horizontal rule is drawn, and the tabs are on the vertical divider
