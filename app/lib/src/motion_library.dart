// The user's motion library (spec 22): captured motions (BVH files)
// shared by every project. Add files, rename or delete motions, or play
// one on the avatar (the project then keeps a copy of it).

import 'package:file_selector/file_selector.dart';
import 'package:material_ui/material_ui.dart';

import 'design/tokens.dart';
import 'i18n.dart';
import 'rust/api/engine.dart';
import 'rust/api/library.dart';

Future<void> showMotionLibrary(
  BuildContext context, {
  required String libraryDir,
  required ValueChanged<String> onPlay,
}) {
  return showDialog<void>(
    context: context,
    builder: (_) => Dialog(
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(kRadius * 1.4),
      ),
      child: SizedBox(
        width: 480,
        child: _MotionLibrary(libraryDir: libraryDir, onPlay: onPlay),
      ),
    ),
  );
}

class _MotionLibrary extends StatefulWidget {
  const _MotionLibrary({required this.libraryDir, required this.onPlay});

  final String libraryDir;
  final ValueChanged<String> onPlay;

  @override
  State<_MotionLibrary> createState() => _MotionLibraryState();
}

class _MotionLibraryState extends State<_MotionLibrary> {
  List<String> _names = const [];
  String? _error;

  /// The motion being renamed, if any.
  String? _renaming;
  final _name = TextEditingController();

  @override
  void initState() {
    super.initState();
    _names = libraryMotions(libraryDir: widget.libraryDir);
  }

  @override
  void dispose() {
    _name.dispose();
    super.dispose();
  }

  void _reload({String? error}) => setState(() {
    _names = libraryMotions(libraryDir: widget.libraryDir);
    _error = error;
  });

  Future<void> _add() async {
    final files = await openFiles(
      acceptedTypeGroups: [
        XTypeGroup(label: context.l.motionFiles, extensions: const ['bvh']),
      ],
    );
    if (files.isEmpty || !mounted) return;
    String? error;
    for (final f in files) {
      try {
        addLibraryMotion(libraryDir: widget.libraryDir, path: f.path);
      } on ApiError catch (e) {
        error = '${f.name}: ${context.error(e)}';
      }
    }
    _reload(error: error);
  }

  void _rename(String name) {
    try {
      renameLibraryMotion(
        libraryDir: widget.libraryDir,
        name: name,
        to: _name.text,
      );
      _renaming = null;
      _reload();
    } on ApiError catch (e) {
      setState(() => _error = context.error(e));
    }
  }

  Future<void> _delete(String name) async {
    final l = context.l;
    final sure = await showDialog<bool>(
      context: context,
      builder: (c) => AlertDialog(
        content: Text(l.motionLibraryDeleteConfirm(name)),
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
      deleteLibraryMotion(libraryDir: widget.libraryDir, name: name);
      _reload();
    } on ApiError catch (e) {
      _reload(error: context.error(e));
    }
  }

  Widget _row(String name) {
    final p = Tokens.of(context);
    final l = context.l;
    if (_renaming == name) {
      return Row(
        key: ValueKey('motionRename:$name'),
        children: [
          Expanded(
            child: TextField(
              controller: _name,
              autofocus: true,
              decoration: const InputDecoration(isDense: true),
              onSubmitted: (_) => _rename(name),
            ),
          ),
          IconButton(
            tooltip: l.rename,
            onPressed: () => _rename(name),
            icon: Icon(Icons.check_rounded, color: p.accent),
          ),
          IconButton(
            tooltip: l.cancel,
            onPressed: () => setState(() => _renaming = null),
            icon: Icon(Icons.close_rounded, color: p.textMuted),
          ),
        ],
      );
    }
    return Row(
      key: ValueKey('motion:$name'),
      children: [
        Icon(Icons.directions_walk_rounded, size: 18, color: p.textMuted),
        const SizedBox(width: Space.s),
        Expanded(
          child: Text(name, overflow: TextOverflow.ellipsis, style: ts()),
        ),
        IconButton(
          key: ValueKey('motionPlay:$name'),
          tooltip: l.motionLibraryPlay,
          onPressed: () {
            Navigator.pop(context);
            widget.onPlay(name);
          },
          icon: Icon(Icons.play_arrow_rounded, color: p.accent),
        ),
        IconButton(
          tooltip: l.rename,
          onPressed: () => setState(() {
            _renaming = name;
            _name.text = name;
            _error = null;
          }),
          icon: Icon(Icons.edit_outlined, size: 19, color: p.textMuted),
        ),
        IconButton(
          tooltip: l.delete,
          onPressed: () => _delete(name),
          icon: Icon(
            Icons.delete_outline_rounded,
            size: 19,
            color: p.textMuted,
          ),
        ),
      ],
    );
  }

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    return Padding(
      padding: const EdgeInsets.all(Space.xl),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Icon(Icons.directions_walk_rounded, color: p.accent),
              const SizedBox(width: Space.s),
              Text(
                l.motionLibrary,
                style: ts(fontSize: 20, fontWeight: FontWeight.w700),
              ),
              const Spacer(),
              IconButton(
                tooltip: l.close,
                onPressed: () => Navigator.pop(context),
                icon: Icon(Icons.close_rounded, color: p.textMuted),
              ),
            ],
          ),
          Text(
            l.motionLibrarySubtitle,
            style: ts(fontSize: 13, color: p.textMuted, height: 1.4),
          ),
          const SizedBox(height: Space.l),
          if (_names.isEmpty)
            Text(
              l.motionLibraryEmpty,
              style: ts(fontSize: 13, color: p.textMuted, height: 1.4),
            )
          else
            ConstrainedBox(
              constraints: const BoxConstraints(maxHeight: 360),
              child: ListView(
                shrinkWrap: true,
                children: [for (final n in _names) _row(n)],
              ),
            ),
          if (_error != null) ...[
            const SizedBox(height: Space.s),
            Text(_error!, style: ts(fontSize: 13, color: p.danger)),
          ],
          const SizedBox(height: Space.l),
          Align(
            alignment: Alignment.centerRight,
            child: FilledButton.icon(
              key: const ValueKey('motionLibraryAdd'),
              onPressed: _add,
              icon: const Icon(Icons.add_rounded, size: 18),
              label: Text(l.motionLibraryAdd),
            ),
          ),
        ],
      ),
    );
  }
}
