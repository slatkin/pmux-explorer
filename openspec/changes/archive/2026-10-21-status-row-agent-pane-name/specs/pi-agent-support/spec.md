# Spec Delta

## MODIFIED Requirements

### Requirement: a subagent pane carries a badge

The status row of a subagent pane SHALL show the badge ♟ for a spawned
subagent and the badge ↻ for a resumed subagent, in the place where a
permission mode badge is shown for other agents.

#### Scenario: A spawned subagent

- **WHEN** a spawned subagent pane is listed
- **THEN** its status row shows the badge ♟

#### Scenario: A resumed subagent

- **WHEN** a resumed subagent pane is listed
- **THEN** its status row shows the badge ↻

#### Scenario: A pi pane that is not a subagent

- **WHEN** a pi pane without a subagent identity is listed
- **THEN** its status row shows no badge
