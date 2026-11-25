// The pattern library (spec 32): the built-in patterns and the user's own,
// by kind, each with a picture of its pieces; one click adds a pattern to
// the project as a new garment drafted for the body. The user's patterns
// are renamed, exported, deleted and imported here; a garment is saved to
// the library from its card.

import 'dart:convert';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';

import 'design/tokens.dart';
import 'i18n.dart';
import 'rust/api/engine.dart';
import 'rust/api/library.dart';
import 'rust/api/templates.dart';
import 'start_screen.dart' show templateText;

/// The kinds the library sorts patterns by, in order.
const _kinds = ['tops', 'bottoms', 'dresses_and_skirts', 'accessories'];

String _kindName(L l, String kind) => switch (kind) {
  'tops' => l.libraryKindTops,
  'bottoms' => l.libraryKindBottoms,
  'dresses_and_skirts' => l.libraryKindDresses,
  'accessories' => l.libraryKindAccessories,
  _ => kind,
};

/// Show the library; [add] runs the step that adds a pattern
/// (`template.apply` or `library.add`).
Future<void> showPatternLibrary(
  BuildContext context, {
  required Engine engine,
  required String libraryDir,
  required Future<void> Function(Map<String, Object?> step) add,
}) {
  return showDialog<void>(
    context: context,
    builder: (_) => Dialog(
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(kRadius * 1.4),
      ),
      child: SizedBox(
        width: 760,
        child: _PatternLibrary(
          engine: engine,
          libraryDir: libraryDir,
          add: add,
        ),
      ),
    ),
  );
}

class _PatternLibrary extends StatefulWidget {
  const _PatternLibrary({
    required this.engine,
    required this.libraryDir,
    required this.add,
  });

  final Engine engine;
  final String libraryDir;
  final Future<void> Function(Map<String, Object?> step) add;

  @override
  State<_PatternLibrary> createState() => _PatternLibraryState();
}

class _PatternLibraryState extends State<_PatternLibrary> {
  /// `template.list`: the built-in patterns.
  List<Map<String, dynamic>> _builtin = const [];

  /// The user's patterns.
  List<Map<String, dynamic>> _mine = const [];
  String? _error;
  final Map<String, Future<Uint8List>> _thumbs = {};

  @override
  void initState() {
    super.initState();
    try {
      _builtin = (jsonDecode(
        widget.engine.run(stepJson: jsonEncode({'type': 'template.list'})),
      ) as List).cast<Map<String, dynamic>>();
    } on ApiError {
      _builtin = const [];
    }
    _reload();
  }

  void _reload({String? error}) => setState(() {
    try {
      _mine = (jsonDecode(
        libraryPatterns(libraryDir: widget.libraryDir),
      ) as List).cast<Map<String, dynamic>>();
      _error = error;
    } on ApiError catch (e) {
      _error = context.error(e);
    }
    _thumbs.clear();
  });

  CanvasColors get _colors =>
      (Theme.of(context).brightness == Brightness.dark
              ? Palette.dark
              : Palette.light)
          .toCanvas();

  Future<Uint8List> _thumb(String key, Future<Uint8List> Function() draw) =>
      _thumbs.putIfAbsent(key, draw);

  Future<void> _add(Map<String, Object?> step) async {
    Navigator.pop(context);
    await widget.add(step);
  }

  Future<void> _import() async {
    final file = await openFile(
      acceptedTypeGroups: [
        XTypeGroup(
          label: context.l.libraryPatternFiles,
          extensions: const ['zip'],
        ),
      ],
    );
    if (file == null || !mounted) return;
    try {
      await importLibraryPattern(
        libraryDir: widget.libraryDir,
        archive: file.path,
      );
      _reload();
    } on ApiError catch (e) {
      if (mounted) _reload(error: context.error(e));
    }
  }

  Future<void> _export(String name) async {
    final to = await getSaveLocation(
      suggestedName: '$name.zip',
      acceptedTypeGroups: [
        XTypeGroup(
          label: context.l.libraryPatternFiles,
          extensions: const ['zip'],
        ),
      ],
    );
    if (to == null || !mounted) return;
    try {
      await exportLibraryPattern(
        libraryDir: widget.libraryDir,
        name: name,
        archive: to.path,
      );
    } on ApiError catch (e) {
      if (mounted) _reload(error: context.error(e));
    }
  }

  Future<void> _rename(String name) async {
    final to = await _askName(context, context.l.rename, name);
    if (to == null || !mounted) return;
    try {
      renameLibraryPattern(libraryDir: widget.libraryDir, name: name, to: to);
      _reload();
    } on ApiError catch (e) {
      _reload(error: context.error(e));
    }
  }

  Future<void> _delete(String name) async {
    final l = context.l;
    final sure = await showDialog<bool>(
      context: context,
      builder: (c) => AlertDialog(
        content: Text(l.libraryDeleteConfirm(name)),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(c, false),
            child: Text(l.cancel),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(c, true),
            child: Text(l.delete),
          ),
        ],
      ),
    );
    if (sure != true || !mounted) return;
    try {
      deleteLibraryPattern(libraryDir: widget.libraryDir, name: name);
      _reload();
    } on ApiError catch (e) {
      _reload(error: context.error(e));
    }
  }

  Widget _tile({
    required Key key,
    required String title,
    required Future<Uint8List> picture,
    required VoidCallback onAdd,
    Widget? menu,
  }) {
    final p = Tokens.of(context);
    return SizedBox(
      key: key,
      width: 164,
      child: Material(
        color: p.surface,
        shape: RoundedRectangleBorder(
          side: BorderSide(color: p.border),
          borderRadius: BorderRadius.circular(Tokens.radius(context)),
        ),
        clipBehavior: Clip.antiAlias,
        child: InkWell(
          onTap: onAdd,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              AspectRatio(
                aspectRatio: 1.6,
                child: FutureBuilder<Uint8List>(
                  future: picture,
                  builder: (_, s) => s.hasData
                      ? Image.memory(s.data!, fit: BoxFit.cover)
                      : ColoredBox(color: p.border.withValues(alpha: 0.3)),
                ),
              ),
              Padding(
                padding: const EdgeInsets.only(
                  left: Space.s,
                  top: Space.xs,
                  bottom: Space.xs,
                ),
                child: Row(
                  children: [
                    Expanded(
                      child: Text(
                        title,
                        overflow: TextOverflow.ellipsis,
                        style: ts(fontSize: 13, fontWeight: FontWeight.w600),
                      ),
                    ),
                    menu ?? const SizedBox(height: 32),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _builtinTile(Map<String, dynamic> t) {
    final id = t['template'] as String;
    final dark = Theme.of(context).brightness == Brightness.dark;
    return _tile(
      key: ValueKey('libraryBuiltin:$id'),
      title: templateText(context.l, id).$1,
      picture: _thumb(
        'template:$id:$dark',
        () => templateThumbnail(
          template: id,
          width: 328,
          height: 205,
          colors: _colors,
        ),
      ),
      onAdd: () => _add({'type': 'template.apply', 'template': id}),
    );
  }

  Widget _mineTile(Map<String, dynamic> m) {
    final l = context.l;
    final name = m['name'] as String;
    final path = m['path'] as String;
    final dark = Theme.of(context).brightness == Brightness.dark;
    return _tile(
      key: ValueKey('libraryMine:$name'),
      title: name,
      picture: _thumb(
        'mine:$path:$dark',
        () => projectThumbnail(
          path: path,
          width: 328,
          height: 205,
          colors: _colors,
        ),
      ),
      onAdd: () => _add({'type': 'library.add', 'name': name}),
      menu: PopupMenuButton<String>(
        key: ValueKey('libraryMenu:$name'),
        icon: const Icon(Icons.more_vert_rounded, size: 18),
        onSelected: (choice) => switch (choice) {
          'rename' => _rename(name),
          'export' => _export(name),
          _ => _delete(name),
        },
        itemBuilder: (_) => [
          PopupMenuItem(value: 'rename', child: Text(l.rename)),
          PopupMenuItem(value: 'export', child: Text(l.libraryExport)),
          PopupMenuItem(value: 'delete', child: Text(l.delete)),
        ],
      ),
    );
  }

  Widget _section(String title, List<Widget> tiles) {
    final p = Tokens.of(context);
    return Padding(
      padding: const EdgeInsets.only(top: Space.l),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            title,
            style: ts(
              fontSize: 13,
              fontWeight: FontWeight.w600,
              color: p.textMuted,
            ),
          ),
          const SizedBox(height: Space.s),
          Wrap(spacing: Space.m, runSpacing: Space.m, children: tiles),
        ],
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    final kinds = [
      for (final k in _kinds)
        if (_builtin.any((t) => t['kind'] == k)) k,
    ];
    final mineKinds = [
      for (final k in _kinds)
        if (_mine.any((m) => m['kind'] == k)) k,
    ];
    return Padding(
      padding: const EdgeInsets.all(Space.xl),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Icon(Icons.collections_bookmark_outlined, color: p.accent),
              const SizedBox(width: Space.s),
              Text(
                l.libraryTitle,
                style: ts(fontSize: 20, fontWeight: FontWeight.w700),
              ),
              const Spacer(),
              TextButton.icon(
                key: const ValueKey('libraryImport'),
                onPressed: _import,
                icon: const Icon(Icons.file_open_outlined, size: 18),
                label: Text(l.libraryImport),
              ),
              IconButton(
                tooltip: l.close,
                onPressed: () => Navigator.pop(context),
                icon: Icon(Icons.close_rounded, color: p.textMuted),
              ),
            ],
          ),
          Text(
            l.librarySubtitle,
            style: ts(fontSize: 13, color: p.textMuted, height: 1.4),
          ),
          if (_error != null) ...[
            const SizedBox(height: Space.s),
            Text(_error!, style: ts(fontSize: 13, color: p.danger)),
          ],
          Flexible(
            child: SingleChildScrollView(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  _section(l.libraryMine, [
                    if (_mine.isEmpty)
                      Text(
                        l.libraryMineEmpty,
                        style: ts(
                          fontSize: 13,
                          color: p.textMuted,
                          height: 1.4,
                        ),
                      ),
                  ]),
                  for (final k in mineKinds)
                    _section('${l.libraryMine} · ${_kindName(l, k)}', [
                      for (final m in _mine)
                        if (m['kind'] == k) _mineTile(m),
                    ]),
                  for (final k in kinds)
                    _section(_kindName(l, k), [
                      for (final t in _builtin)
                        if (t['kind'] == k) _builtinTile(t),
                    ]),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}

/// Ask for a name, starting from [initial]; `null` if cancelled.
Future<String?> _askName(
  BuildContext context,
  String title,
  String initial,
) async {
  final name = TextEditingController(text: initial);
  final out = await showDialog<String>(
    context: context,
    builder: (c) => AlertDialog(
      title: Text(title),
      content: TextField(
        controller: name,
        autofocus: true,
        onSubmitted: (v) => Navigator.pop(c, v),
      ),
      actions: [
        TextButton(onPressed: () => Navigator.pop(c), child: Text(c.l.cancel)),
        FilledButton(
          onPressed: () => Navigator.pop(c, name.text),
          child: Text(c.l.save),
        ),
      ],
    ),
  );
  name.dispose();
  return out;
}

/// Save [garment] (as `project.summary` lists it) to the library under a
/// name and a kind the user picks; a name already there is replaced only
/// once the user agrees. Returns a message to show, if any.
Future<String?> saveToLibrary(
  BuildContext context,
  Engine engine,
  Map<String, dynamic> garment,
) async {
  final picked = await showDialog<(String, String)>(
    context: context,
    builder: (_) => _SaveDialog(initial: garment['name'] as String? ?? ''),
  );
  if (picked == null || !context.mounted) return null;
  final (name, kind) = picked;
  Map<String, Object?> step(bool replace) => {
    'type': 'library.save',
    'garment': garment['id'],
    'name': name,
    'kind': kind,
    'replace': replace,
  };
  try {
    engine.run(stepJson: jsonEncode(step(false)));
  } on ApiError catch (e) {
    if (e.code != 'library.pattern_exists') return context.error(e);
    final l = context.l;
    final replace = await showDialog<bool>(
      context: context,
      builder: (c) => AlertDialog(
        content: Text(l.libraryReplaceConfirm(name)),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(c, false),
            child: Text(l.cancel),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(c, true),
            child: Text(l.libraryReplace),
          ),
        ],
      ),
    );
    if (replace != true || !context.mounted) return null;
    try {
      engine.run(stepJson: jsonEncode(step(true)));
    } on ApiError catch (e) {
      return context.mounted ? context.error(e) : null;
    }
  }
  return context.mounted ? context.l.librarySaved(name) : null;
}

/// A name and a kind for a garment saved to the library.
class _SaveDialog extends StatefulWidget {
  const _SaveDialog({required this.initial});

  final String initial;

  @override
  State<_SaveDialog> createState() => _SaveDialogState();
}

class _SaveDialogState extends State<_SaveDialog> {
  late final _name = TextEditingController(text: widget.initial);
  String _kind = _kinds.first;

  @override
  void dispose() {
    _name.dispose();
    super.dispose();
  }

  void _done() => Navigator.pop(context, (_name.text.trim(), _kind));

  @override
  Widget build(BuildContext context) {
    final l = context.l;
    return AlertDialog(
      title: Text(l.librarySaveTitle),
      content: SizedBox(
        width: 360,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            TextField(
              key: const ValueKey('librarySaveName'),
              controller: _name,
              autofocus: true,
              decoration: InputDecoration(labelText: l.librarySaveName),
              onSubmitted: (_) => _done(),
            ),
            const SizedBox(height: Space.m),
            DropdownButtonFormField<String>(
              key: const ValueKey('librarySaveKind'),
              initialValue: _kind,
              decoration: InputDecoration(labelText: l.librarySaveKind),
              items: [
                for (final k in _kinds)
                  DropdownMenuItem(value: k, child: Text(_kindName(l, k))),
              ],
              onChanged: (k) => setState(() => _kind = k ?? _kind),
            ),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: Text(l.cancel),
        ),
        FilledButton(
          key: const ValueKey('librarySaveDone'),
          onPressed: _done,
          child: Text(l.save),
        ),
      ],
    );
  }
}
