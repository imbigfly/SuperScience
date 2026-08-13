# Workspace navigation

By default, opening a workspace restores its most recently active conversation.
Opening a specific conversation from Recent sessions or search still takes
priority.

Choose another workspace from the sidebar workspace menu to switch the current
window in place. A separate window is opened only by an action explicitly
labelled **Open in new window**.

To open workspaces on a blank conversation instead, turn off **Resume the last
conversation when opening a workspace** in **Settings → General**. Starting a
new conversation manually is always available from the sidebar. A newly
created conversation appears there immediately as **Untitled session**, even
before its first message is sent.

Use the magnifying-glass button beside **Sessions** to search conversation
titles in the current workspace. Search includes older conversations that have
not been loaded into the paginated sidebar yet. Clear the field or press Escape
to restore the normal grouped conversation list.

## Project rules changes and existing conversations

A conversation's system prompt — including `AGENTS.md` and the project **Agent
context** (`.wisp/WISP.md`) — is assembled once when the conversation starts
and kept stable for its lifetime, so edits apply only to new conversations.
When the files on disk no longer match a conversation's persisted prompt, the
sidebar marks that conversation with a circular-refresh icon in the left status
slot (the same gutter used for running and waiting-for-you). Right-click it and
choose **Reload project rules…** to rebuild its system prompt from the current
files. The reload takes effect on the next turn and leaves the chat history
untouched; because the prompt prefix changes, the provider's prompt cache for
that conversation is invalidated once, so the next turn costs a bit more.
