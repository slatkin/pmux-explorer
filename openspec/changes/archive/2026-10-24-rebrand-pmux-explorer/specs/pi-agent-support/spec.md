# Spec Delta

## MODIFIED Requirements

### Requirement: the producer protocol is documented

The repository SHALL carry a document that states, for every event of the
protocol, the JSON fields a producer sends and what the sidebar does with them.
The `setup` subcommand SHALL accept the agent name `pi` and SHALL print that
contract instead of merging a settings file, because pi has no hook
configuration file.

#### Scenario: The protocol document exists
- **WHEN** the repository is read
- **THEN** a document states the event names, the fields of each event, and the meaning of each field

#### Scenario: The setup command for pi
- **WHEN** `pmux-explorer setup pi` runs
- **THEN** it prints the producer contract and exits successfully, and it changes no configuration file
