// The start screen: start a garment from a template (drafted to the body's
// measurements), start blank, reopen a recent project, or rename, export,
// delete and import whole projects (docs/ux.md §1, docs/specs/06 and 11).

import 'dart:convert';

import 'package:flutter/services.dart';
import 'package:file_selector/file_selector.dart';
import 'package:intl/intl.dart';
import 'package:material_ui/material_ui.dart';
import 'package:url_launcher/url_launcher.dart';

import 'design/tokens.dart';
import 'i18n.dart';
import 'busy.dart';
import 'projects.dart';
import 'rust/api/engine.dart';
import 'rust/api/library.dart';
import 'rust/api/templates.dart';
import 'support.dart';
import 'update.dart';

/// What the start screen offers, in order: the `template.apply` ids.
const _templates = [
  ('t_shirt', Icons.checkroom_rounded),
  ('shirt', Icons.dry_cleaning_outlined),
  ('trousers', Icons.accessibility_new_rounded),
  ('hoodie', Icons.checkroom_outlined),
  ('joggers', Icons.directions_run_rounded),
  ('jeans', Icons.accessibility_rounded),
  ('shorts', Icons.wb_sunny_outlined),
  ('dress', Icons.dry_cleaning_rounded),
  ('straight_skirt', Icons.straighten_rounded),
  ('a_line_skirt', Icons.change_history_rounded),
  ('tote_bag', Icons.shopping_bag_outlined),
];

/// A template's title and blurb (also the pattern library's, spec 32).
(String, String) templateText(L l, String id) => switch (id) {
  't_shirt' => (l.templateTShirt, l.templateTShirtBlurb),
  'shirt' => (l.templateShirt, l.templateShirtBlurb),
  'trousers' => (l.templateTrousers, l.templateTrousersBlurb),
  'hoodie' => (l.templateHoodie, l.templateHoodieBlurb),
  'joggers' => (l.templateJoggers, l.templateJoggersBlurb),
  'jeans' => (l.templateJeans, l.templateJeansBlurb),
  'shorts' => (l.templateShorts, l.templateShortsBlurb),
  'dress' => (l.templateDress, l.templateDressBlurb),
  'straight_skirt' => (l.templateStraightSkirt, l.templateStraightSkirtBlurb),
  'a_line_skirt' => (l.templateALineSkirt, l.templateALineSkirtBlurb),
  'tote_bag' => (l.templateToteBag, l.templateToteBagBlurb),
  _ => (id, ''),
};

/// A size a template reads: its label and hint.
(String, String) _sizeText(L l, String key) => switch (key) {
  'chest_cm' => (l.sizeChest, l.sizeChestHint),
  'waist_cm' => (l.sizeWaist, l.sizeWaistHint),
  'hips_cm' => (l.sizeHips, l.sizeHipsHint),
  'length_cm' => (l.sizeLength, l.sizeLengthHint),
  'sleeve_cm' => (l.sizeSleeve, l.sizeSleeveHint),
  'width_cm' => (l.sizeWidth, l.sizeWidthHint),
  'ease_cm' => (l.sizeEase, l.sizeEaseHint),
  _ => (key, ''),
};

class StartScreen extends StatefulWidget {
  const StartScreen({
    super.key,
    required this.store,
    required this.support,
    this.updates,
    required this.dark,
    required this.onToggleTheme,
    required this.language,
    required this.onLanguage,
    required this.onOpen,
  });

  final ProjectStore store;

  /// Whether this launch shows the donation card.
  final SupportMemory support;

  /// The update check, none in tests.
  final UpdateCheck? updates;

  final bool dark;
  final VoidCallback onToggleTheme;

  /// The UI language's code, and how to change it.
  final String language;
  final ValueChanged<String> onLanguage;

  /// Open the project folder at this path.
  final ValueChanged<String> onOpen;

  @override
  State<StartScreen> createState() => _StartScreenState();
}

/// How many projects the start screen shows; the others are on the
/// "All projects" page.
const _recentCount = 4;

/// Where updates and news are published.
final _website = Uri.parse('https://rustle3d.com');

/// Pictures of patterns (templates and projects), cached per theme.
class _Thumbs {
  final Map<(String, bool), Future<Uint8List>> _cache = {};

  static CanvasColors _colors(bool dark) =>
      (dark ? Palette.dark : Palette.light).toCanvas();

  Future<Uint8List> template(String id, bool dark) => _cache.putIfAbsent(
    ('template:$id', dark),
    () => templateThumbnail(
      template: id,
      width: 480,
      height: 300,
      colors: _colors(dark),
    ),
  );

  /// Redrawn when the project changes.
  Future<Uint8List> project(RecentProject p, bool dark) => _cache.putIfAbsent(
    ('project:${p.path}:${p.modified.millisecondsSinceEpoch}', dark),
    () => projectThumbnail(
      path: p.path,
      width: 480,
      height: 300,
      colors: _colors(dark),
    ),
  );
}

void _toast(BuildContext context, String text) {
  ScaffoldMessenger.of(context)
    ..hideCurrentSnackBar()
    ..showSnackBar(
      SnackBar(content: Text(text), behavior: SnackBarBehavior.floating),
    );
}

/// Show [target] (a folder or a web page) with the system.
Future<void> _launch(BuildContext context, Uri target) async {
  final failed = context.l.cannotOpen(target.toString());
  final ok = await launchUrl(target);
  if (!ok && context.mounted) _toast(context, failed);
}

/// Ask for a project folder (`.r3d`) anywhere on disk.
Future<String?> _pickProject(BuildContext context) =>
    getDirectoryPath(confirmButtonText: context.l.open);

/// Ask for a project archive (`.zip`) and unpack it into the projects
/// folder; returns the new project's folder.
Future<String?> _importZip(BuildContext context, ProjectStore store) async {
  final file = await openFile(
    acceptedTypeGroups: [
      XTypeGroup(label: context.l.projectZipFiles, extensions: const ['zip']),
    ],
  );
  if (file == null || !context.mounted) return null;
  try {
    final path = await store.importZip(file.path);
    store.remember(path);
    return path;
  } on ApiError catch (e) {
    if (context.mounted) _toast(context, context.error(e));
    return null;
  }
}

/// The "Import .zip…" button: the imported project opens.
class _ImportZipButton extends StatelessWidget {
  const _ImportZipButton({required this.store, required this.onOpen});

  final ProjectStore store;
  final ValueChanged<String> onOpen;

  @override
  Widget build(BuildContext context) {
    return Tooltip(
      message: context.l.importProjectZipTip,
      child: TextButton.icon(
        onPressed: () async {
          final path = await _importZip(context, store);
          if (path != null) onOpen(path);
        },
        icon: const Icon(Icons.unarchive_outlined, size: 18),
        label: Text(context.l.importProjectZip),
      ),
    );
  }
}

class _StartScreenState extends State<StartScreen> {
  late List<RecentProject> _projects = widget.store.all();
  final _thumbs = _Thumbs();

  void _updatesChanged() => setState(() {});

  @override
  void initState() {
    super.initState();
    widget.updates?.addListener(_updatesChanged);
  }

  @override
  void dispose() {
    widget.updates?.removeListener(_updatesChanged);
    super.dispose();
  }

  Future<void> _newGarment(String template, String title) async {
    final path = await showDialog<String>(
      context: context,
      builder: (_) => _NewGarmentDialog(
        store: widget.store,
        template: template,
        title: title,
      ),
    );
    if (path != null) widget.onOpen(path);
  }

  Future<void> _blank() async {
    try {
      final path = await widget.store.create(
        name: context.l.untitled,
        body: widget.store.standardBody(),
      );
      widget.onOpen(path);
    } on ApiError catch (e) {
      if (mounted) _toast(context, context.error(e));
    }
  }

  Future<void> _openFromDisk() async {
    final path = await _pickProject(context);
    if (path != null) widget.onOpen(path);
  }

  Future<void> _allProjects() async {
    await Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) => _AllProjectsPage(
          store: widget.store,
          thumbs: _thumbs,
          dark: widget.dark,
          onOpen: (path) {
            Navigator.of(context).pop();
            widget.onOpen(path);
          },
        ),
      ),
    );
    // Projects may have been renamed or deleted there.
    _refresh();
  }

  void _refresh() {
    if (mounted) setState(() => _projects = widget.store.all());
  }

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    return Scaffold(
      body: Column(
        children: [
          SizedBox(
            height: 56,
            child: Padding(
              padding: const EdgeInsets.only(left: 84, right: Space.m),
              child: Row(
                children: [
                  const Logo(),
                  const SizedBox(width: Space.m),
                  Text(
                    'Rustle3D',
                    style: ts(
                      fontSize: 17,
                      fontWeight: FontWeight.w700,
                      color: p.text,
                    ),
                  ),
                  const Spacer(),
                  LanguageButton(
                    current: widget.language,
                    onChanged: widget.onLanguage,
                  ),
                  IconButton(
                    tooltip: widget.dark ? l.themeLight : l.themeDark,
                    onPressed: () {
                      widget.onToggleTheme();
                      setState(() {});
                    },
                    icon: Icon(
                      widget.dark
                          ? Icons.light_mode_outlined
                          : Icons.dark_mode_outlined,
                      size: 19,
                      color: p.textMuted,
                    ),
                  ),
                ],
              ),
            ),
          ),
          Expanded(
            child: SingleChildScrollView(
              padding: const EdgeInsets.fromLTRB(
                Space.xl * 2,
                Space.xl,
                Space.xl * 2,
                Space.xl * 2,
              ),
              child: Center(
                child: ConstrainedBox(
                  constraints: const BoxConstraints(maxWidth: 1080),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        l.startTitle,
                        style: ts(
                          fontSize: 30,
                          fontWeight: FontWeight.w700,
                          color: p.text,
                          letterSpacing: -0.5,
                        ),
                      ),
                      if (widget.updates case final updates?)
                        if (updates.available case final version?)
                          UpdateCard(
                            version: version,
                            onDownload: () =>
                                _launch(context, downloadUrl(widget.language)),
                            onSkip: updates.skip,
                          ),
                      if (widget.support.remindNow)
                        SupportCard(
                          onDonate: () {
                            setState(widget.support.later);
                            _launch(context, donateUrl);
                          },
                          onLater: () => setState(widget.support.later),
                          onTurnOff: () => setState(widget.support.turnOff),
                        ),
                      _SectionTitle(
                        l.recent,
                        actions: [
                          TextButton.icon(
                            onPressed: _openFromDisk,
                            icon: const Icon(
                              Icons.folder_open_outlined,
                              size: 18,
                            ),
                            label: Text(l.openFromDisk),
                          ),
                          _ImportZipButton(
                            store: widget.store,
                            onOpen: widget.onOpen,
                          ),
                          if (_projects.length > _recentCount)
                            TextButton.icon(
                              onPressed: _allProjects,
                              icon: const Icon(
                                Icons.grid_view_rounded,
                                size: 18,
                              ),
                              label: Text(l.allProjectsCount(_projects.length)),
                            ),
                        ],
                      ),
                      if (_projects.isEmpty)
                        Text(
                          l.recentEmpty,
                          style: ts(fontSize: 13, color: p.textMuted),
                        )
                      else
                        _ProjectGrid(
                          projects: _projects.take(_recentCount),
                          store: widget.store,
                          thumbs: _thumbs,
                          dark: widget.dark,
                          onOpen: widget.onOpen,
                          onChanged: _refresh,
                        ),
                      _SectionTitle(l.sectionNew),
                      Wrap(
                        spacing: Space.l,
                        runSpacing: Space.l,
                        children: [
                          for (final (id, icon) in _templates)
                            if (templateText(l, id) case (
                              final title,
                              final blurb,
                            ))
                              _PictureCard(
                                title: title,
                                subtitle: blurb,
                                icon: icon,
                                picture: _thumbs.template(id, widget.dark),
                                onTap: () => _newGarment(id, title),
                              ),
                          _BlankCard(onTap: _blank),
                        ],
                      ),
                      _SectionTitle('Rustle3D'),
                      Wrap(
                        spacing: Space.s,
                        runSpacing: Space.s,
                        crossAxisAlignment: WrapCrossAlignment.center,
                        children: [
                          Text(
                            l.appVersion(appVersion()),
                            style: ts(fontSize: 13, color: p.textMuted),
                          ),
                          const SizedBox(width: Space.s),
                          TextButton.icon(
                            onPressed: () => _launch(context, _website),
                            icon: const Icon(
                              Icons.open_in_new_rounded,
                              size: 16,
                            ),
                            label: Text(l.updatesLink),
                          ),
                          if (widget.updates case final updates?)
                            TextButton.icon(
                              key: const ValueKey('updatesButton'),
                              onPressed: () => showUpdates(
                                context,
                                updates: updates,
                                onDownload: () => _launch(
                                  context,
                                  downloadUrl(widget.language),
                                ),
                              ),
                              icon: const Icon(
                                Icons.system_update_alt_rounded,
                                size: 16,
                              ),
                              label: Text(l.updateCheck),
                            ),
                          TextButton.icon(
                            onPressed: () => _launch(context, donateUrl),
                            icon: const Icon(
                              Icons.favorite_border_rounded,
                              size: 16,
                            ),
                            label: Text(l.supportLink),
                          ),
                          TextButton.icon(
                            onPressed: () => _launch(
                              context,
                              Uri.directory(widget.store.projectsDir),
                            ),
                            icon: const Icon(Icons.folder_outlined, size: 16),
                            label: Text(l.projectsFolder),
                          ),
                        ],
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }

  @override
  void didUpdateWidget(StartScreen old) {
    super.didUpdateWidget(old);
    _projects = widget.store.all();
  }
}

/// A section's title, with its actions on the right.
class _SectionTitle extends StatelessWidget {
  const _SectionTitle(this.title, {this.actions = const []});

  final String title;
  final List<Widget> actions;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    return Padding(
      padding: const EdgeInsets.only(top: Space.xl * 1.5, bottom: Space.m),
      child: Row(
        children: [
          Text(
            title,
            style: ts(fontSize: 17, fontWeight: FontWeight.w600, color: p.text),
          ),
          const Spacer(),
          ...actions,
        ],
      ),
    );
  }
}

/// Projects as cards: a picture of the pattern, the name, when it changed,
/// and a menu to rename, export or delete the project.
class _ProjectGrid extends StatelessWidget {
  const _ProjectGrid({
    required this.projects,
    required this.store,
    required this.thumbs,
    required this.dark,
    required this.onOpen,
    required this.onChanged,
  });

  final Iterable<RecentProject> projects;
  final ProjectStore store;
  final _Thumbs thumbs;
  final bool dark;
  final ValueChanged<String> onOpen;

  /// A project was renamed or deleted.
  final VoidCallback onChanged;

  @override
  Widget build(BuildContext context) {
    return Wrap(
      spacing: Space.l,
      runSpacing: Space.l,
      children: [
        for (final project in projects)
          _PictureCard(
            title: project.name,
            subtitle: _ago(context, project.modified),
            icon: Icons.folder_outlined,
            picture: thumbs.project(project, dark),
            onTap: () => onOpen(project.path),
            menu: _ProjectMenu(
              project: project,
              store: store,
              onChanged: onChanged,
            ),
          ),
      ],
    );
  }
}

enum _ProjectAction { rename, export, delete }

/// A project card's menu: rename, export as .zip, delete.
class _ProjectMenu extends StatelessWidget {
  const _ProjectMenu({
    required this.project,
    required this.store,
    required this.onChanged,
  });

  final RecentProject project;
  final ProjectStore store;
  final VoidCallback onChanged;

  Future<void> _rename(BuildContext context) async {
    final renamed = await showDialog<bool>(
      context: context,
      builder: (_) => _RenameProjectDialog(project: project, store: store),
    );
    if (renamed == true) onChanged();
  }

  Future<void> _export(BuildContext context) async {
    final l = context.l;
    final location = await getSaveLocation(
      suggestedName: '${project.name}.zip',
      acceptedTypeGroups: [
        XTypeGroup(label: l.projectZipFiles, extensions: const ['zip']),
      ],
    );
    if (location == null || !context.mounted) return;
    final path = location.path.toLowerCase().endsWith('.zip')
        ? location.path
        : '${location.path}.zip';
    try {
      await store.exportZip(project.path, path);
      if (context.mounted) {
        _toast(context, l.projectExported(project.name, path));
      }
    } on ApiError catch (e) {
      if (context.mounted) _toast(context, context.error(e));
    }
  }

  Future<void> _delete(BuildContext context) async {
    final l = context.l;
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (_) => _DeleteProjectDialog(name: project.name),
    );
    if (confirmed != true || !context.mounted) return;
    try {
      store.delete(project.path);
      _toast(context, l.projectDeleted(project.name));
      onChanged();
    } on ApiError catch (e) {
      _toast(context, context.error(e));
    }
  }

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    return PopupMenuButton<_ProjectAction>(
      tooltip: l.projectOptions,
      icon: Icon(Icons.more_horiz_rounded, size: 18, color: p.textMuted),
      padding: EdgeInsets.zero,
      onSelected: (action) => switch (action) {
        _ProjectAction.rename => _rename(context),
        _ProjectAction.export => _export(context),
        _ProjectAction.delete => _delete(context),
      },
      itemBuilder: (_) => [
        for (final (action, icon, label, color) in [
          (
            _ProjectAction.rename,
            Icons.drive_file_rename_outline_rounded,
            l.renameProject,
            p.text,
          ),
          (
            _ProjectAction.export,
            Icons.archive_outlined,
            l.exportProjectZip,
            p.text,
          ),
          (
            _ProjectAction.delete,
            Icons.delete_outline_rounded,
            l.deleteProject,
            p.danger,
          ),
        ])
          PopupMenuItem(
            value: action,
            child: Row(
              children: [
                Icon(icon, size: 18, color: color),
                const SizedBox(width: Space.s),
                Text(label, style: ts(color: color)),
              ],
            ),
          ),
      ],
    );
  }
}

/// A small dialog: a title, the content, then the buttons on the right.
class _SmallDialog extends StatelessWidget {
  const _SmallDialog({
    required this.title,
    required this.children,
    required this.actions,
  });

  final String title;
  final List<Widget> children;
  final List<Widget> actions;

  @override
  Widget build(BuildContext context) {
    return Dialog(
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(kRadius * 1.4),
      ),
      child: SizedBox(
        width: 440,
        child: Padding(
          padding: const EdgeInsets.all(Space.xl),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(title, style: ts(fontSize: 20, fontWeight: FontWeight.w700)),
              const SizedBox(height: Space.l),
              ...children,
              const SizedBox(height: Space.xl),
              Row(
                children: [
                  const Spacer(),
                  for (final (i, action) in actions.indexed) ...[
                    if (i > 0) const SizedBox(width: Space.s),
                    action,
                  ],
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// A new name for a project; pops `true` once renamed.
class _RenameProjectDialog extends StatefulWidget {
  const _RenameProjectDialog({required this.project, required this.store});

  final RecentProject project;
  final ProjectStore store;

  @override
  State<_RenameProjectDialog> createState() => _RenameProjectDialogState();
}

class _RenameProjectDialogState extends State<_RenameProjectDialog> {
  late final _name = TextEditingController(text: widget.project.name)
    ..selection = TextSelection(
      baseOffset: 0,
      extentOffset: widget.project.name.length,
    );
  String? _error;

  @override
  void dispose() {
    _name.dispose();
    super.dispose();
  }

  void _rename() {
    final name = _name.text.trim();
    if (name == widget.project.name) {
      Navigator.pop(context, false);
      return;
    }
    try {
      widget.store.rename(widget.project.path, name);
      Navigator.pop(context, true);
    } on ApiError catch (e) {
      setState(() => _error = context.error(e));
    }
  }

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    return _SmallDialog(
      title: l.renameProjectTitle,
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context, false),
          child: Text(l.cancel),
        ),
        FilledButton(onPressed: _rename, child: Text(l.rename)),
      ],
      children: [
        TextField(
          controller: _name,
          autofocus: true,
          onSubmitted: (_) => _rename(),
          decoration: InputDecoration(
            labelText: l.fieldName,
            border: const OutlineInputBorder(),
          ),
        ),
        if (_error != null) ...[
          const SizedBox(height: Space.m),
          Text(_error!, style: ts(fontSize: 13, color: p.danger)),
        ],
      ],
    );
  }
}

/// Asks before deleting a project: the Delete button stays off until the
/// box saying it is for good is ticked. Pops `true` to delete.
class _DeleteProjectDialog extends StatefulWidget {
  const _DeleteProjectDialog({required this.name});

  final String name;

  @override
  State<_DeleteProjectDialog> createState() => _DeleteProjectDialogState();
}

class _DeleteProjectDialogState extends State<_DeleteProjectDialog> {
  bool _understood = false;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    return _SmallDialog(
      title: l.deleteProjectTitle(widget.name),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context, false),
          child: Text(l.cancel),
        ),
        FilledButton.icon(
          style: FilledButton.styleFrom(backgroundColor: p.danger),
          onPressed: _understood ? () => Navigator.pop(context, true) : null,
          icon: const Icon(Icons.delete_outline_rounded, size: 18),
          label: Text(l.delete),
        ),
      ],
      children: [
        Text(
          l.deleteProjectBody,
          style: ts(fontSize: 14, color: p.textMuted, height: 1.4),
        ),
        const SizedBox(height: Space.m),
        CheckboxListTile(
          value: _understood,
          onChanged: (v) => setState(() => _understood = v ?? false),
          title: Text(l.deleteProjectConfirm, style: ts(fontSize: 14)),
          controlAffinity: ListTileControlAffinity.leading,
          contentPadding: EdgeInsets.zero,
        ),
      ],
    );
  }
}

/// Every project, with opening one from anywhere on disk.
class _AllProjectsPage extends StatefulWidget {
  const _AllProjectsPage({
    required this.store,
    required this.thumbs,
    required this.dark,
    required this.onOpen,
  });

  final ProjectStore store;
  final _Thumbs thumbs;
  final bool dark;
  final ValueChanged<String> onOpen;

  @override
  State<_AllProjectsPage> createState() => _AllProjectsPageState();
}

class _AllProjectsPageState extends State<_AllProjectsPage> {
  late List<RecentProject> _projects = widget.store.all();

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    final store = widget.store;
    final onOpen = widget.onOpen;
    return Scaffold(
      body: Column(
        children: [
          SizedBox(
            height: 56,
            child: Padding(
              padding: const EdgeInsets.only(left: 84, right: Space.m),
              child: Row(
                children: [
                  IconButton(
                    onPressed: () => Navigator.of(context).pop(),
                    icon: Icon(Icons.arrow_back_rounded, color: p.textMuted),
                  ),
                  const SizedBox(width: Space.s),
                  Text(
                    l.allProjects,
                    style: ts(
                      fontSize: 17,
                      fontWeight: FontWeight.w700,
                      color: p.text,
                    ),
                  ),
                  const Spacer(),
                  TextButton.icon(
                    onPressed: () async {
                      final path = await _pickProject(context);
                      if (path != null) onOpen(path);
                    },
                    icon: const Icon(Icons.folder_open_outlined, size: 18),
                    label: Text(l.openFromDisk),
                  ),
                  _ImportZipButton(store: store, onOpen: onOpen),
                  TextButton.icon(
                    onPressed: () =>
                        _launch(context, Uri.directory(store.projectsDir)),
                    icon: const Icon(Icons.folder_outlined, size: 18),
                    label: Text(l.projectsFolder),
                  ),
                ],
              ),
            ),
          ),
          Expanded(
            child: SingleChildScrollView(
              padding: const EdgeInsets.all(Space.xl * 2),
              child: Center(
                child: ConstrainedBox(
                  constraints: const BoxConstraints(maxWidth: 1080),
                  child: SizedBox(
                    width: double.infinity,
                    child: _projects.isEmpty
                        ? Text(
                            l.recentEmpty,
                            style: ts(fontSize: 13, color: p.textMuted),
                          )
                        : _ProjectGrid(
                            projects: _projects,
                            store: store,
                            thumbs: widget.thumbs,
                            dark: widget.dark,
                            onOpen: onOpen,
                            onChanged: () =>
                                setState(() => _projects = store.all()),
                          ),
                  ),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

/// The app's mark: a piece of cloth in two panels sewn down the middle.
/// Drawn from `assets/brand/mark-small.svg` (the version for small sizes),
/// in its 1024 units.
class Logo extends StatelessWidget {
  const Logo({super.key});

  @override
  Widget build(BuildContext context) {
    return const CustomPaint(size: Size.square(26), painter: _MarkPainter());
  }
}

class _MarkPainter extends CustomPainter {
  const _MarkPainter();

  static const _tileTop = Color(0xFF7B5CFF);
  static const _tileBottom = Color(0xFF5634E6);
  static const _leftPanel = Color(0xFFD9D0FF);
  static const _stitch = Color(0xFFD9409A);

  /// The hem's two scallops from the centre to the left edge.
  static void _leftHem(Path p) => p
    ..cubicTo(487, 796, 389, 796, 364, 750)
    ..cubicTo(339, 796, 241, 796, 216, 750);

  @override
  void paint(Canvas canvas, Size size) {
    canvas.scale(size.width / 1024);
    const square = Rect.fromLTWH(0, 0, 1024, 1024);
    canvas.drawRRect(
      RRect.fromRectAndRadius(square, const Radius.circular(228)),
      Paint()
        ..shader = const LinearGradient(
          begin: Alignment.topCenter,
          end: Alignment.bottomCenter,
          colors: [_tileTop, _tileBottom],
        ).createShader(square),
    );
    canvas
      ..translate(512, 512)
      ..scale(1.1)
      ..translate(-512, -529);
    final cloth = Path()
      ..moveTo(256, 262)
      ..lineTo(768, 262)
      ..lineTo(808, 750)
      ..cubicTo(783, 796, 685, 796, 660, 750)
      ..cubicTo(635, 796, 537, 796, 512, 750);
    _leftHem(cloth);
    canvas.drawPath(cloth..close(), Paint()..color = Colors.white);
    final left = Path()
      ..moveTo(256, 262)
      ..lineTo(512, 262)
      ..lineTo(512, 750);
    _leftHem(left);
    canvas.drawPath(left..close(), Paint()..color = _leftPanel);
    final stitch = Paint()..color = _stitch;
    for (final y in const [330.0, 470.0, 610.0]) {
      canvas.drawRRect(
        RRect.fromRectAndRadius(
          Rect.fromLTWH(486, y, 52, 92),
          const Radius.circular(26),
        ),
        stitch,
      );
    }
  }

  @override
  bool shouldRepaint(_MarkPainter oldDelegate) => false;
}

/// A hoverable card frame.
class _Card extends StatefulWidget {
  const _Card({required this.onTap, required this.child, this.dashed = false});

  final VoidCallback onTap;
  final Widget child;
  final bool dashed;

  @override
  State<_Card> createState() => _CardState();
}

class _CardState extends State<_Card> {
  bool _hover = false;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    return MouseRegion(
      cursor: SystemMouseCursors.click,
      onEnter: (_) => setState(() => _hover = true),
      onExit: (_) => setState(() => _hover = false),
      child: GestureDetector(
        onTap: widget.onTap,
        child: AnimatedContainer(
          // Eases in, but leaves at once: moving from one card to the next
          // never shows two highlighted.
          duration: _hover ? Motion.fast : Duration.zero,
          curve: Motion.curve,
          width: 248,
          height: 268,
          transform: Matrix4.translationValues(0, _hover ? -2 : 0, 0),
          decoration: BoxDecoration(
            color: widget.dashed ? Colors.transparent : p.surface,
            borderRadius: BorderRadius.circular(kRadius),
            border: Border.all(
              color: _hover ? p.accent.withValues(alpha: 0.6) : p.border,
            ),
            boxShadow: [
              if (!widget.dashed)
                BoxShadow(
                  color: Colors.black.withValues(alpha: _hover ? 0.08 : 0.03),
                  blurRadius: _hover ? 18 : 8,
                  offset: const Offset(0, 4),
                ),
            ],
          ),
          clipBehavior: Clip.antiAlias,
          child: widget.child,
        ),
      ),
    );
  }
}

/// A card with a picture of a pattern, a title and a line under it.
class _PictureCard extends StatelessWidget {
  const _PictureCard({
    required this.title,
    required this.subtitle,
    required this.icon,
    required this.picture,
    required this.onTap,
    this.menu,
  });

  final String title;
  final String subtitle;
  final IconData icon;
  final Future<Uint8List> picture;
  final VoidCallback onTap;

  /// Shown at the end of the title's line.
  final Widget? menu;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    return _Card(
      onTap: onTap,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Container(
            height: 155,
            width: double.infinity,
            color: p.board,
            child: FutureBuilder<Uint8List>(
              future: picture,
              builder: (context, snap) => AnimatedOpacity(
                opacity: snap.hasData ? 1 : 0,
                duration: Motion.normal,
                child: snap.hasData
                    ? Image.memory(snap.data!, fit: BoxFit.cover)
                    : const SizedBox.expand(),
              ),
            ),
          ),
          Padding(
            padding: const EdgeInsets.all(Space.l),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Icon(icon, size: 18, color: p.accent),
                    const SizedBox(width: Space.s),
                    Expanded(
                      child: Text(
                        title,
                        overflow: TextOverflow.ellipsis,
                        style: ts(
                          fontSize: 15,
                          fontWeight: FontWeight.w600,
                          color: p.text,
                        ),
                      ),
                    ),
                    if (menu case final menu?)
                      SizedBox.square(dimension: 24, child: menu),
                  ],
                ),
                const SizedBox(height: 6),
                Text(
                  subtitle,
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                  style: ts(fontSize: 12.5, color: p.textMuted, height: 1.35),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _BlankCard extends StatelessWidget {
  const _BlankCard({required this.onTap});

  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    return _Card(
      onTap: onTap,
      dashed: true,
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Container(
            width: 44,
            height: 44,
            decoration: BoxDecoration(
              color: p.accentSoft,
              shape: BoxShape.circle,
            ),
            child: Icon(Icons.add_rounded, color: p.accent),
          ),
          const SizedBox(height: Space.m),
          Text(
            context.l.blankProject,
            style: ts(fontSize: 15, fontWeight: FontWeight.w600, color: p.text),
          ),
          const SizedBox(height: 6),
          Text(
            context.l.blankProjectBlurb,
            style: ts(fontSize: 12.5, color: p.textMuted),
          ),
        ],
      ),
    );
  }
}

String _ago(BuildContext context, DateTime t) {
  final l = context.l;
  final d = DateTime.now().difference(t);
  if (d.inMinutes < 1) return l.agoJustNow;
  if (d.inHours < 1) return l.agoMinutes(d.inMinutes);
  if (d.inDays < 1) return l.agoHours(d.inHours);
  if (d.inDays == 1) return l.agoYesterday;
  if (d.inDays < 30) return l.agoDays(d.inDays);
  return DateFormat.yMMMd(Localizations.localeOf(context).toString()).format(t);
}

/// Name the garment, pick the body, check the sizes, create.
class _NewGarmentDialog extends StatefulWidget {
  const _NewGarmentDialog({
    required this.store,
    required this.template,
    required this.title,
  });

  final ProjectStore store;
  final String template;
  final String title;

  @override
  State<_NewGarmentDialog> createState() => _NewGarmentDialogState();
}

class _NewGarmentDialogState extends State<_NewGarmentDialog> {
  late final _name = TextEditingController(text: widget.title);
  late final List<Map<String, dynamic>> _presets = (jsonDecode(
    bodyPresets(libraryDir: widget.store.libraryDir),
  ) as List).cast<Map<String, dynamic>>();
  late Map<String, dynamic> _body = widget.store.standardBody();
  final Map<String, TextEditingController> _sizes = {};
  String? _error;

  @override
  void initState() {
    super.initState();
    _loadSizes();
  }

  @override
  void dispose() {
    _name.dispose();
    for (final c in _sizes.values) {
      c.dispose();
    }
    super.dispose();
  }

  /// The template's default sizes for the chosen body.
  void _loadSizes() {
    final list = jsonDecode(templateList(bodyJson: jsonEncode(_body['body'])));
    final entry = (list as List).firstWhere(
      (t) => t['template'] == widget.template,
    );
    final sizes = (entry['sizes'] as Map<String, dynamic>);
    for (final c in _sizes.values) {
      c.dispose();
    }
    _sizes
      ..clear()
      ..addAll({
        for (final e in sizes.entries)
          e.key: TextEditingController(
            text: (e.value as num).toStringAsFixed(
              (e.value as num) % 1 == 0 ? 0 : 1,
            ),
          ),
      });
  }

  Future<void> _create() async {
    final sizes = <String, double>{};
    for (final e in _sizes.entries) {
      final v = double.tryParse(e.value.text.replaceAll(',', '.'));
      if (v == null || v <= 0) {
        setState(
          () => _error = context.l.sizeMustBePositive(
            _sizeText(context.l, e.key).$1,
          ),
        );
        return;
      }
      sizes[e.key] = v;
    }
    final name = _name.text.trim().isEmpty ? widget.title : _name.text.trim();
    try {
      final path = await widget.store.create(
        name: name,
        body: _body,
        template: widget.template,
        sizes: sizes,
        pieceNames: pieceNames(context.l),
        busy: (engine, job) => runBusy(
          context,
          engine,
          context.l.busyTemplate,
          job,
          cancellable: true,
        ),
      );
      if (mounted) Navigator.pop(context, path);
    } on ApiError catch (e) {
      if (mounted) setState(() => _error = context.error(e));
    }
  }

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    return Dialog(
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(kRadius * 1.4),
      ),
      child: SizedBox(
        width: 520,
        child: Padding(
          padding: const EdgeInsets.all(Space.xl),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  Text(
                    l.newGarmentTitle(widget.title),
                    style: ts(fontSize: 20, fontWeight: FontWeight.w700),
                  ),
                  const Spacer(),
                  IconButton(
                    onPressed: () => Navigator.pop(context),
                    icon: Icon(Icons.close_rounded, color: p.textMuted),
                  ),
                ],
              ),
              const SizedBox(height: Space.m),
              TextField(
                controller: _name,
                decoration: InputDecoration(
                  labelText: l.fieldName,
                  border: const OutlineInputBorder(),
                ),
              ),
              const SizedBox(height: Space.l),
              DropdownButtonFormField<String>(
                initialValue: _body['name'] as String,
                decoration: InputDecoration(
                  labelText: l.fieldBody,
                  border: const OutlineInputBorder(),
                  prefixIcon: const Icon(
                    Icons.accessibility_new_rounded,
                    size: 18,
                  ),
                ),
                items: [
                  for (final x in _presets)
                    DropdownMenuItem(
                      value: x['name'] as String,
                      child: Text(context.presetName(x)),
                    ),
                ],
                onChanged: (name) => setState(() {
                  _body = _presets.firstWhere((x) => x['name'] == name);
                  _loadSizes();
                }),
              ),
              const SizedBox(height: Space.l),
              Wrap(
                spacing: Space.m,
                runSpacing: Space.m,
                children: [
                  for (final e in _sizes.entries)
                    SizedBox(
                      width: 224,
                      child: Tooltip(
                        message: _sizeText(l, e.key).$2,
                        child: TextField(
                          controller: e.value,
                          keyboardType: const TextInputType.numberWithOptions(
                            decimal: true,
                          ),
                          inputFormatters: [
                            FilteringTextInputFormatter.allow(
                              RegExp(r'[0-9.,]'),
                            ),
                          ],
                          onSubmitted: (_) => _create(),
                          decoration: InputDecoration(
                            labelText: _sizeText(l, e.key).$1,
                            suffixText: l.cm,
                            border: const OutlineInputBorder(),
                          ),
                        ),
                      ),
                    ),
                ],
              ),
              if (_error != null) ...[
                const SizedBox(height: Space.m),
                Text(_error!, style: ts(fontSize: 13, color: p.danger)),
              ],
              const SizedBox(height: Space.xl),
              Row(
                children: [
                  const Spacer(),
                  TextButton(
                    onPressed: () => Navigator.pop(context),
                    child: Text(l.cancel),
                  ),
                  const SizedBox(width: Space.s),
                  FilledButton.icon(
                    onPressed: _create,
                    icon: const Icon(Icons.auto_awesome_rounded, size: 18),
                    label: Text(l.create),
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }
}
