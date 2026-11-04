# Documentation

Yolo's public documentation is organized by the boundary it explains. Dated
validation notes and internal release evidence are not part of the public tree.

## Start here

- [Alpha contract](alpha.md): support matrix, included workflows and limits.
- [Architecture](architecture.md): process boundaries and ownership.
- [Concept model](concept-model.md): terminals, agents, runs and authority.
- [Operations](operations.md): install, launch, diagnostics and removal.
- [Local service lifecycle](local-service-lifecycle.md): status, start, stop,
  restart and release activation.

## Runtime and protocol

- [Single-executable architecture](single-executable-architecture.md)
- [Protocol](protocol.md) and [public JSON-RPC](public-jsonrpc.md)
- [Client resource protocol](client-resource-protocol.md)
- [Persistence](persistence.md) and [PTY environment](pty-environment.md)
- [Terminal and agent model](terminal-agent-model.md)
- [Terminal input control](terminal-input-control.md)
- [Terminal presentation](terminal-presentation.md)
- [Socket namespace lifecycle](socket-namespace-lifecycle.md)
- [Runtime bundle catalog](runtime-bundle-catalog.md)

## Hub, remote and federation

- [Communication architecture](communication-architecture.md)
- [Hub client channels](hub-client-channels.md)
- [Hub inventory](hub-inventory.md) and [offline inventory](hub-offline-inventory.md)
- [Hub management](hub-management.md) and [local startup](hub-local-startup.md)
- [Hub pairing](hub-pairing.md)
- [Hub agent control](hub-agent-control.md) and [agent directory](hub-agent-directory.md)
- [Hub native control](hub-native-control-execution.md)
- [Hub native workspaces](hub-native-workspaces.md)
- [Hub terminal relay](hub-terminal-relay.md) and [multipath](hub-multipath.md)
- [Hub remote setup](hub-remote-setup.md) and [SSH profiles](hub-ssh-profiles.md)
- [Remote Hub delivery](remote-hub-delivery.md)
- [Socket namespace lifecycle](socket-namespace-lifecycle.md)
- [Token freshness and upgrade admission](upgrade-admission.md)

## TUI

- [Interactions](tui-interactions.md) and [tab navigation](tui-tab-navigation.md)
- [Tabs](tui-tabs.md) and [workspaces](tui-workspaces.md)
- [Local startup](tui-local-startup.md) and [remote entry](tui-remote-entry.md)
- [Creation recovery](tui-creation-recovery.md) and [settlement](tui-creation-settlement.md)
- [Terminal queries](tui-terminal-queries.md)
- [Chat clients](chat-clients.md) and [native shared layouts](native-shared-layouts.md)

## Agents and providers

- [Provider extension contract](provider-extension-contract.md)
- [Provider runtime capabilities](provider-runtime-capabilities.md)
- [Provider hooks research](provider-hooks-research.md)
- [Conversation reader contract](conversation-reader-contract.md)
- [Creation query origin](creation-query-origin.md)
- [Protocol creation transaction](protocol-creation-transaction.md)

## Plugins

- [Trusted plugin protocol](trusted-plugin-protocol.md)
- [Client plugin contract](client-plugin-contract.md)
- [Web client](https://github.com/AgentsMesh/YoloShellWeb)
- [Plugin registry](https://github.com/AgentsMesh/YoloShellPlugins)
- [ADR 0023: trusted-only plugins](adr/0023-trusted-only-plugins.md)
- [ADR 0024: web terminal streaming](adr/0024-web-terminal-streaming.md)
- [ADR 0026: web session broker](adr/0026-web-session-broker.md)
- [ADR 0027: durable web creation](adr/0027-durable-web-creation.md)
- [ADR 0029: single broker data directory](adr/0029-single-broker-data-directory.md)
- [ADR 0030: web terminal lifecycle](adr/0030-web-terminal-lifecycle.md)
- [ADR 0033: web client extraction and plugin registry](adr/0033-web-client-extraction-and-plugin-registry.md)
- [ADR 0034: mechanisms, first-party clients and external plugins](adr/0034-mechanisms-clients-plugins.md)
- [ADR 0035: remove the yolo web gateway](adr/0035-remove-yolo-web-gateway.md)

## Decisions

- [Architecture decision records](adr/) — numbered decisions with context,
  alternatives and consequences.
- [Unreleased contract policy](unreleased-contract-policy.md)
- [Release resource custody](release-resource-custody.md)
- [Hub file management](hub-file-root-management.md) and [files](hub-files.md)
- [Hub connector boundary](hub-connector-boundary.md)
- [Hub session metadata](hub-session-metadata.md)

## Repository

- [Release pipeline](release-pipeline.md)
- [Contributing](../CONTRIBUTING.md)
- [Security policy](../SECURITY.md)
- [Third-party notices](../THIRD_PARTY_NOTICES.md)
- [Architecture gates](../README.md#architecture-gates)
