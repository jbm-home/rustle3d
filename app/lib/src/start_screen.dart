// The start screen: start a garment from a template (drafted to the body's
// measurements), start blank, or reopen a recent project (docs/ux.md §1,
// docs/specs/11).

import 'dart:convert';

import 'package:flutter/services.dart';
import 'package:intl/intl.dart';
import 'package:material_ui/material_ui.dart';

import 'design/tokens.dart';
import 'i18n.dart';
import 'projects.dart';
import 'rust/api/engine.dart';
import 'rust/api/library.dart';
import 'rust/api/templates.dart';

/// What the start screen offers, in order: the `template.apply` ids.
const _templates = [
  ('t_shirt', Icons.checkroom_rounded),
  ('dress', Icons.dry_cleaning_rounded),
  ('straight_skirt', Icons.straighten_rounded),
  ('a_line_skirt', Icons.change_history_rounded),
  ('tote_bag', Icons.shopping_bag_outlined),
];

/// A template's title and blurb.
(String, String) _templateText(L l, String id) => switch (id) {
  't_shirt' => (l.templateTShirt, l.templateTShirtBlurb),
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
    required this.dark,
    required this.onToggleTheme,
    required this.language,
    required this.onLanguage,
    required this.onOpen,
  });

  final ProjectStore store;
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

class _StartScreenState extends State<StartScreen> {
  late List<RecentProject> _recent = widget.store.recent();
  final Map<(String, bool), Future<Uint8List>> _thumbs = {};

  Future<Uint8List> _thumb(String template) {
    return _thumbs.putIfAbsent(
      (template, widget.dark),
      () => templateThumbnail(
        template: template,
        width: 480,
        height: 300,
        colors: (widget.dark ? Palette.dark : Palette.light).toCanvas(),
      ),
    );
  }

  void _toast(String text) {
    ScaffoldMessenger.of(context)
      ..hideCurrentSnackBar()
      ..showSnackBar(
        SnackBar(content: Text(text), behavior: SnackBarBehavior.floating),
      );
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
      final path = widget.store.create(
        name: context.l.untitled,
        body: widget.store.standardBody(),
      );
      widget.onOpen(path);
    } on ApiError catch (e) {
      _toast(context.error(e));
    }
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
                      const SizedBox(height: Space.s),
                      Text(
                        l.startSubtitle,
                        style: ts(
                          fontSize: 15,
                          color: p.textMuted,
                          height: 1.4,
                        ),
                      ),
                      const SizedBox(height: Space.xl),
                      Wrap(
                        spacing: Space.l,
                        runSpacing: Space.l,
                        children: [
                          for (final (id, icon) in _templates)
                            if (_templateText(l, id) case (
                              final title,
                              final blurb,
                            ))
                              _TemplateCard(
                                title: title,
                                blurb: blurb,
                                icon: icon,
                                thumb: _thumb(id),
                                onTap: () => _newGarment(id, title),
                              ),
                          _BlankCard(onTap: _blank),
                        ],
                      ),
                      const SizedBox(height: Space.xl * 1.5),
                      Text(
                        l.recent,
                        style: ts(
                          fontSize: 17,
                          fontWeight: FontWeight.w600,
                          color: p.text,
                        ),
                      ),
                      const SizedBox(height: Space.m),
                      if (_recent.isEmpty)
                        Text(
                          l.recentEmpty,
                          style: ts(fontSize: 13, color: p.textMuted),
                        )
                      else
                        for (final r in _recent.take(8))
                          _RecentRow(
                            project: r,
                            onTap: () => widget.onOpen(r.path),
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
    _recent = widget.store.recent();
  }
}

/// The app's mark.
class Logo extends StatelessWidget {
  const Logo({super.key});

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    return Container(
      width: 26,
      height: 26,
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(7),
        gradient: LinearGradient(
          colors: [
            p.accent,
            Color.lerp(p.accent, const Color(0xFFE879A6), 0.6)!,
          ],
          begin: Alignment.topLeft,
          end: Alignment.bottomRight,
        ),
      ),
      alignment: Alignment.center,
      child: Text(
        'R',
        style: ts(
          color: Colors.white,
          fontWeight: FontWeight.w700,
          fontSize: 14,
        ),
      ),
    );
  }
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
          duration: Motion.fast,
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

class _TemplateCard extends StatelessWidget {
  const _TemplateCard({
    required this.title,
    required this.blurb,
    required this.icon,
    required this.thumb,
    required this.onTap,
  });

  final String title;
  final String blurb;
  final IconData icon;
  final Future<Uint8List> thumb;
  final VoidCallback onTap;

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
              future: thumb,
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
                    Text(
                      title,
                      style: ts(
                        fontSize: 15,
                        fontWeight: FontWeight.w600,
                        color: p.text,
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 6),
                Text(
                  blurb,
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

class _RecentRow extends StatefulWidget {
  const _RecentRow({required this.project, required this.onTap});

  final RecentProject project;
  final VoidCallback onTap;

  @override
  State<_RecentRow> createState() => _RecentRowState();
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

class _RecentRowState extends State<_RecentRow> {
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
          duration: Motion.fast,
          padding: const EdgeInsets.symmetric(
            horizontal: Space.m,
            vertical: Space.m,
          ),
          decoration: BoxDecoration(
            color: _hover ? p.surfaceMuted : Colors.transparent,
            borderRadius: BorderRadius.circular(kRadius * 0.6),
          ),
          child: Row(
            children: [
              Icon(Icons.folder_outlined, size: 18, color: p.textMuted),
              const SizedBox(width: Space.m),
              Text(
                widget.project.name,
                style: ts(
                  fontSize: 14,
                  fontWeight: FontWeight.w500,
                  color: p.text,
                ),
              ),
              const Spacer(),
              Text(
                _ago(context, widget.project.modified),
                style: ts(fontSize: 12.5, color: p.textMuted),
              ),
            ],
          ),
        ),
      ),
    );
  }
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

  void _create() {
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
      final path = widget.store.create(
        name: name,
        body: _body,
        template: widget.template,
        sizes: sizes,
        pieceNames: pieceNames(context.l),
      );
      Navigator.pop(context, path);
    } on ApiError catch (e) {
      setState(() => _error = context.error(e));
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
              Text(
                l.newGarmentSubtitle,
                style: ts(fontSize: 13, color: p.textMuted, height: 1.4),
              ),
              const SizedBox(height: Space.l),
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
