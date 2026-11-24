// Connecting an AI agent to the open project: allow it, add Rustle3D to the
// agent's MCP servers (the command with r3d's full path, ready to copy, or
// an extension file for desktop assistants), and see it connect.

import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';

import 'design/tokens.dart';
import 'i18n.dart';
import 'rust/api/engine.dart';

Future<void> showAgents(
  BuildContext context, {
  required Engine engine,
  required ValueChanged<bool> onAllow,
  required VoidCallback onDisconnect,
}) {
  return showDialog<void>(
    context: context,
    builder: (_) =>
        _Agents(engine: engine, onAllow: onAllow, onDisconnect: onDisconnect),
  );
}

/// `r3d`, the command-line tool the packages ship next to the app; `null`
/// when it is not there (a build from source).
String? _r3dPath() {
  final name = Platform.isWindows ? 'r3d.exe' : 'r3d';
  final path = File(Platform.resolvedExecutable).parent.uri
      .resolve(name)
      .toFilePath();
  return File(path).existsSync() ? path : null;
}

/// How an agent's configuration names the server: settings to copy, or an
/// extension file (`.mcpb`) to install.
enum _Client { json, claudeCode, codex, extension }

class _Agents extends StatefulWidget {
  const _Agents({
    required this.engine,
    required this.onAllow,
    required this.onDisconnect,
  });

  final Engine engine;
  final ValueChanged<bool> onAllow;
  final VoidCallback onDisconnect;

  @override
  State<_Agents> createState() => _AgentsState();
}

class _AgentsState extends State<_Agents> {
  final String? _r3d = _r3dPath();
  _Client _client = _Client.json;
  AgentStatus _status = AgentStatus.off;
  bool _copied = false;

  /// Where the extension was saved, or why it could not be.
  String? _saved;
  String? _saveError;
  Timer? _poll;

  @override
  void initState() {
    super.initState();
    _read();
    _poll = Timer.periodic(const Duration(milliseconds: 300), (_) => _read());
  }

  @override
  void dispose() {
    _poll?.cancel();
    super.dispose();
  }

  void _read() {
    AgentStatus status;
    try {
      status = widget.engine.agentStatus();
    } catch (_) {
      status = AgentStatus.off;
    }
    if (mounted && status != _status) setState(() => _status = status);
  }

  String _config() {
    final command = _r3d ?? 'r3d';
    return switch (_client) {
      // Laid out by hand: the encoder puts each argument on its own line.
      _Client.json =>
        '{\n'
            '  "mcpServers": {\n'
            '    "rustle3d": {\n'
            '      "command": ${jsonEncode(command)},\n'
            '      "args": ["mcp", "--attach"]\n'
            '    }\n'
            '  }\n'
            '}',
      _Client.claudeCode =>
        'claude mcp add rustle3d -- ${_quoted(command)} mcp --attach',
      _Client.codex =>
        '[mcp_servers.rustle3d]\n'
            'command = ${jsonEncode(command)}\n'
            'args = ["mcp", "--attach"]',
      _Client.extension => '',
    };
  }

  Future<void> _saveExtension(String r3d) async {
    final location = await getSaveLocation(
      suggestedName: 'rustle3d.mcpb',
      acceptedTypeGroups: [
        const XTypeGroup(label: 'MCP Bundle', extensions: ['mcpb']),
      ],
    );
    if (location == null || !mounted) return;
    try {
      saveAgentExtension(r3D: r3d, out: location.path);
      setState(() {
        _saved = location.path;
        _saveError = null;
      });
    } on ApiError catch (e) {
      setState(() {
        _saved = null;
        _saveError = context.error(e);
      });
    }
  }

  static String _quoted(String s) => s.contains(' ') ? '"$s"' : s;

  Future<void> _copy() async {
    await Clipboard.setData(ClipboardData(text: _config()));
    if (!mounted) return;
    setState(() => _copied = true);
    Timer(const Duration(seconds: 2), () {
      if (mounted) setState(() => _copied = false);
    });
  }

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    Widget step(int n, String title, List<Widget> body) => Padding(
      padding: const EdgeInsets.only(top: Space.l),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          CircleAvatar(
            radius: 11,
            backgroundColor: p.accentSoft,
            child: Text(
              '$n',
              style: ts(
                fontSize: 12,
                fontWeight: FontWeight.w600,
                color: p.accent,
              ),
            ),
          ),
          const SizedBox(width: Space.m),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Text(
                  title,
                  style: ts(
                    fontSize: 13.5,
                    fontWeight: FontWeight.w600,
                    color: p.text,
                  ),
                ),
                ...body,
              ],
            ),
          ),
        ],
      ),
    );
    Widget hint(String text) => Padding(
      padding: const EdgeInsets.only(top: Space.xs),
      child: Text(
        text,
        style: ts(fontSize: 12.5, height: 1.45, color: p.textMuted),
      ),
    );
    final allowed = _status != AgentStatus.off;
    final connected = _status == AgentStatus.connected;
    return AlertDialog(
      title: Text(l.agentsTitle),
      content: SizedBox(
        width: 520,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text(
                l.agentsIntro,
                style: ts(fontSize: 12.5, height: 1.45, color: p.textMuted),
              ),
              step(1, l.agentsStepAllow, [
                SwitchListTile(
                  key: const ValueKey('agentsAllow'),
                  contentPadding: EdgeInsets.zero,
                  dense: true,
                  title: Text(l.allowAgents, style: ts(fontSize: 13)),
                  value: allowed,
                  onChanged: widget.onAllow,
                ),
              ]),
              step(2, l.agentsStepAdd, [
                hint(l.agentsAddHint),
                const SizedBox(height: Space.s),
                SegmentedButton<_Client>(
                  key: const ValueKey('agentsClient'),
                  segments: [
                    ButtonSegment(value: _Client.json, label: Text('JSON')),
                    const ButtonSegment(
                      value: _Client.claudeCode,
                      label: Text('Claude Code'),
                    ),
                    const ButtonSegment(
                      value: _Client.codex,
                      label: Text('Codex'),
                    ),
                    const ButtonSegment(
                      value: _Client.extension,
                      label: Text('Claude Desktop'),
                    ),
                  ],
                  selected: {_client},
                  onSelectionChanged: (s) => setState(() => _client = s.first),
                ),
                if (_client == _Client.json) hint(l.agentsJsonHint),
                if (_client == _Client.extension) ...[
                  hint(l.agentsExtensionHint),
                  const SizedBox(height: Space.s),
                  Align(
                    alignment: Alignment.centerLeft,
                    child: OutlinedButton.icon(
                      key: const ValueKey('agentsSaveExtension'),
                      icon: const Icon(Icons.download, size: 18),
                      label: Text(l.agentsSaveExtension),
                      onPressed: switch (_r3d) {
                        final r3d? => () => _saveExtension(r3d),
                        null => null,
                      },
                    ),
                  ),
                  if (_saved case final saved?)
                    hint(l.agentsExtensionSaved(saved)),
                  if (_saveError case final error?) hint(error),
                ] else ...[
                  const SizedBox(height: Space.s),
                  Container(
                    padding: const EdgeInsets.all(Space.m),
                    decoration: BoxDecoration(
                      color: p.surfaceMuted,
                      border: Border.all(color: p.border),
                      borderRadius: BorderRadius.circular(kRadius),
                    ),
                    child: Row(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Expanded(
                          child: SelectableText(
                            _config(),
                            key: const ValueKey('agentsConfig'),
                            style: ts(fontSize: 12, height: 1.4, color: p.text),
                          ),
                        ),
                        IconButton(
                          key: const ValueKey('agentsCopy'),
                          tooltip: l.agentsCopy,
                          icon: Icon(
                            _copied ? Icons.check : Icons.content_copy,
                            size: 18,
                          ),
                          onPressed: _copy,
                        ),
                      ],
                    ),
                  ),
                ],
                if (_r3d == null) hint(l.agentsNoR3d),
              ]),
              step(3, l.agentsStepAsk, [
                hint(l.agentsAskHint),
                const SizedBox(height: Space.s),
                Row(
                  children: [
                    Icon(
                      connected ? Icons.link : Icons.hourglass_empty,
                      size: 16,
                      color: allowed ? p.accent : p.textMuted,
                    ),
                    const SizedBox(width: Space.s),
                    Expanded(
                      child: Text(
                        switch (_status) {
                          AgentStatus.off => l.agentsStateOff,
                          AgentStatus.waiting => l.agentsStateWaiting,
                          AgentStatus.connected => l.agentConnected,
                        },
                        key: const ValueKey('agentsState'),
                        style: ts(
                          fontSize: 13,
                          fontWeight: FontWeight.w500,
                          color: allowed ? p.accent : p.textMuted,
                        ),
                      ),
                    ),
                    if (connected)
                      TextButton(
                        onPressed: widget.onDisconnect,
                        child: Text(l.disconnectAgent),
                      ),
                  ],
                ),
              ]),
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l.close),
        ),
      ],
    );
  }
}
