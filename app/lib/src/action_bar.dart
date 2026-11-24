// The action bar above the 2D and 3D views (spec 30): what is picked on
// the board, and the constraints, dimensions and actions that fit it.

part of 'shell.dart';

/// One of the bar's buttons: `label` under its icon, `name` and `hint`
/// in its tip.
typedef _BarAction = ({
  String id,
  IconData icon,
  String label,
  String name,
  String hint,
  // Given the button's context, to open a menu under it.
  void Function(BuildContext) onTap,
});

class _ActionBar extends StatelessWidget {
  const _ActionBar({required this.picked, required this.groups});

  /// What is picked ("2 corners · 1 edge", a piece's name); empty:
  /// nothing.
  final String picked;

  /// The buttons, in groups set apart by a line.
  final List<List<_BarAction>> groups;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    Widget tip(String name, String hint, Widget child) => Tooltip(
      richMessage: TextSpan(
        children: [
          TextSpan(
            text: '$name\n',
            style: ts(fontWeight: FontWeight.w600),
          ),
          TextSpan(text: hint),
        ],
      ),
      child: child,
    );
    Widget separator() => Container(
      width: 1,
      height: 32,
      margin: const EdgeInsets.symmetric(horizontal: Space.s),
      color: p.border,
    );
    return Container(
      height: 60,
      margin: const EdgeInsets.fromLTRB(
        Space.s + 52 + Space.s,
        0,
        Space.m,
        Space.s,
      ),
      padding: const EdgeInsets.symmetric(horizontal: Space.s),
      decoration: BoxDecoration(
        color: p.surface,
        borderRadius: BorderRadius.circular(Tokens.radius(context)),
        border: Border.all(color: p.border),
      ),
      // Keys stay with the board (Delete, Enter, the tools).
      child: ExcludeFocus(
        child: Row(
          children: [
            // What is picked; greyed when nothing is, the tip says how.
            tip(
              picked.isEmpty ? l.barNothing : picked,
              context.keys(l.barHint),
              _BarTile(
                icon: Icons.ads_click_rounded,
                label: picked.isEmpty ? l.barNothing : picked,
                enabled: picked.isNotEmpty,
              ),
            ),
            if (groups.isNotEmpty) separator(),
            Expanded(
              child: SingleChildScrollView(
                scrollDirection: Axis.horizontal,
                child: Row(
                  children: [
                    for (final (i, group) in groups.indexed) ...[
                      if (i > 0) separator(),
                      for (final a in group)
                        tip(
                          a.name,
                          a.hint,
                          Builder(
                            builder: (at) => _BarTile(
                              key: ValueKey('bar_${a.id}'),
                              icon: a.icon,
                              label: a.label,
                              onTap: () => a.onTap(at),
                            ),
                          ),
                        ),
                    ],
                  ],
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// A button of the action bar: an icon with a short label under it.
/// Without `onTap`, a plain tile (what is picked), greyed unless
/// `enabled`.
class _BarTile extends StatefulWidget {
  const _BarTile({
    super.key,
    required this.icon,
    required this.label,
    this.onTap,
    this.enabled = true,
  });

  final IconData icon;
  final String label;
  final VoidCallback? onTap;
  final bool enabled;

  @override
  State<_BarTile> createState() => _BarTileState();
}

class _BarTileState extends State<_BarTile> {
  bool _hover = false;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final tap = widget.onTap;
    final color = !widget.enabled
        ? p.textMuted.withValues(alpha: 0.55)
        : tap == null
        ? p.text
        : p.textMuted;
    final tile = AnimatedContainer(
      duration: Motion.fast,
      curve: Motion.curve,
      constraints: const BoxConstraints(minWidth: 52, maxWidth: 160),
      height: 48,
      padding: const EdgeInsets.symmetric(horizontal: 6),
      decoration: BoxDecoration(
        color: tap != null && _hover ? p.surfaceMuted : Colors.transparent,
        borderRadius: BorderRadius.circular(Tokens.radius(context, 0.75)),
      ),
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(widget.icon, size: 22, color: color),
          const SizedBox(height: 3),
          Text(
            widget.label,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: ts(
              fontSize: 10.5,
              fontWeight: tap == null ? FontWeight.w600 : FontWeight.w500,
              color: color,
            ),
          ),
        ],
      ),
    );
    if (tap == null) return tile;
    return Semantics(
      button: true,
      label: widget.label,
      child: MouseRegion(
        onEnter: (_) => setState(() => _hover = true),
        onExit: (_) => setState(() => _hover = false),
        cursor: SystemMouseCursors.click,
        child: GestureDetector(onTap: tap, child: tile),
      ),
    );
  }
}

extension _Bar on _ShellState {
  /// What is picked, in words ("2 corners · 1 edge", or the piece's
  /// name); empty: nothing.
  String _pickedText(L l) {
    final parts = [
      if (_points.isNotEmpty) l.barCorners(_points.length),
      if (_edges.isNotEmpty) l.barEdges(_edges.length),
      if (_notches.isNotEmpty) l.barNotches(_notches.length),
      if (_guides.isNotEmpty) l.barLines(_guides.length),
    ];
    if (parts.isEmpty && _selection.length > 1) {
      return l.piecesSelected(_selection.length);
    }
    if (parts.isEmpty) return _panel?['name'] as String? ?? '';
    return parts.join(' · ');
  }

  /// Add a constraint (one undo step).
  void _constrain(Map<String, Object?> c) {
    _run({'type': 'constraint.add', 'constraint': c});
    _refresh();
  }

  /// Add constraints together, as one undo step.
  void _constrainAll(String label, List<Map<String, Object?>> cs) {
    _run({
      'type': 'batch',
      'label': label,
      'commands': [
        for (final c in cs) {'type': 'constraint.add', 'constraint': c},
      ],
    });
    _refresh();
  }

  /// Add a dimension with the value the drawing has, and open its value
  /// field where its label is drawn.
  void _dimension(Map<String, Object?> c) {
    final out = _run({
      'type': 'constraint.add',
      'constraint': {...c, 'mm': 1},
      'measure': true,
    });
    final id = (out?['created'] as List?)?.firstOrNull as String?;
    _refresh();
    if (id != null) _openField(id);
  }

  /// Turn a construction line about its first end, level or upright.
  void _straightenGuide(String guide, {required bool level}) {
    final g = (_run({'type': 'guide.list'}) as List?)
        ?.cast<Map<String, dynamic>>()
        .where((g) => g['id'] == guide)
        .firstOrNull;
    if (g == null) return;
    final a = _ShellState._at(g['a']);
    final b = _ShellState._at(g['b']);
    final d = b - a;
    final len = d.distance;
    final to = level
        ? a + Offset(d.dx < 0 ? -len : len, 0)
        : a + Offset(0, d.dy < 0 ? -len : len);
    _run({
      'type': 'guide.move',
      'guide': guide,
      'a': [a.dx, a.dy],
      'b': [to.dx, to.dy],
    });
    _refresh();
  }

  /// Join the piece with another, or cut that one out of it
  /// (`panel.boolean`), the other picked in a menu under the button.
  Future<void> _combine(BuildContext at, String piece, String op) async {
    final box = at.findRenderObject();
    final overlay = Overlay.of(at).context.findRenderObject();
    if (box is! RenderBox || overlay is! RenderBox) return;
    final corner = box.localToGlobal(
      Offset(0, box.size.height),
      ancestor: overlay,
    );
    final other = await showMenu<String>(
      context: at,
      position: RelativeRect.fromLTRB(
        corner.dx,
        corner.dy,
        overlay.size.width - corner.dx,
        0,
      ),
      items: [
        for (final o in (_summary['panels'] as List? ?? const []))
          if (o is Map<String, dynamic> && o['id'] != piece)
            PopupMenuItem(
              value: o['id'] as String,
              child: Text(o['name'] as String),
            ),
      ],
    );
    if (other == null || !mounted) return;
    _run({'type': 'panel.boolean', 'panel': piece, 'with': other, 'op': op});
    _refresh();
  }

  /// Sew an elastic along an edge, its relaxed length and width asked.
  Future<void> _addElastic(String edge) async {
    final l = context.l;
    final picked = await _askNumbers(
      context,
      title: l.elasticTitle,
      fields: [(l.elasticRatio, '%', 75), (l.elasticWidth, 'mm', 10)],
    );
    if (picked == null || !mounted) return;
    final [ratio, width] = picked.$1;
    _run({
      'type': 'elastic.add',
      'side': {'edge': edge},
      'ratio': ratio / 100,
      'width_mm': width,
    });
    _refresh();
  }

  /// Shirr a piece: rows of elastic parallel to an edge.
  Future<void> _shirr(String panel, String edge) async {
    final l = context.l;
    final picked = await _askNumbers(
      context,
      title: l.shirringTitle,
      fields: [
        (l.shirringRows, '', 5),
        (l.shirringFirst, l.cm, 1.5),
        (l.shirringGap, l.cm, 1.5),
        (l.elasticRatio, '%', 75),
      ],
    );
    if (picked == null || !mounted) return;
    final [rows, first, gap, ratio] = picked.$1;
    _run({
      'type': 'elastic.shirr',
      'panel': panel,
      'edge': edge,
      'rows': rows.round(),
      'first_mm': first * 10,
      'gap_mm': gap * 10,
      'ratio': ratio / 100,
    });
    _refresh();
  }

  /// The buttons for the piece selected alone, nothing on it picked:
  /// turn, scale, mirror; join or cut out; delete.
  List<List<_BarAction>> _pieceActions(L l, Map<String, dynamic> panel) {
    final id = panel['id'] as String;
    final linked = panel['mirror_of'] != null;
    final others = (_summary['panels'] as List? ?? const []).length > 1;
    return [
      if (!linked)
        [
          (
            id: 'rotate',
            icon: Icons.rotate_left_rounded,
            label: l.barShortRotate,
            name: l.rotatePiece,
            hint: l.barRotateHint,
            onTap: (_) async {
              final picked = await _askNumbers(
                context,
                title: l.rotatePiece,
                fields: [(l.rotateAngle, '°', 90)],
                hint: l.rotatePieceHint,
              );
              final angle = picked?.$1.first ?? 0;
              if (angle == 0) return;
              _run({'type': 'panel.rotate', 'panel': id, 'angle_deg': angle});
              _refresh();
            },
          ),
          (
            id: 'scale',
            icon: Icons.open_in_full_rounded,
            label: l.barShortScale,
            name: l.scalePiece,
            hint: l.barScaleHint,
            onTap: (_) async {
              final picked = await _askNumbers(
                context,
                title: l.scalePiece,
                fields: [(l.scaleAcross, '%', 100), (l.scaleUpDown, '%', 100)],
              );
              if (picked == null) return;
              final [x, y] = picked.$1;
              if (x <= 0 || y <= 0) return;
              _run({
                'type': 'panel.scale',
                'panel': id,
                'factor': [x / 100, y / 100],
              });
              _refresh();
            },
          ),
          (
            id: 'mirror',
            icon: Icons.flip_rounded,
            label: l.mirror,
            name: l.mirror,
            hint: l.barMirrorHint,
            onTap: (_) {
              _run({
                'type': 'panel.mirror',
                'panel': id,
                'name': l.pieceMirror(panel['name'] as String),
              });
              _refresh();
            },
          ),
        ],
      if (!linked && others)
        [
          (
            id: 'join',
            icon: Icons.join_full_rounded,
            label: l.join,
            name: l.join,
            hint: l.barJoinHint,
            onTap: (at) => _combine(at, id, 'union'),
          ),
          (
            id: 'cut_out',
            icon: Icons.join_left_rounded,
            label: l.cutOut,
            name: l.cutOut,
            hint: l.barCutOutHint,
            onTap: (at) => _combine(at, id, 'difference'),
          ),
        ],
      [
        (
          id: 'delete',
          icon: Icons.delete_outline_rounded,
          label: l.delete,
          name: l.delete,
          hint: l.barDeleteHint,
          onTap: (_) => _deleteSelected(),
        ),
      ],
    ];
  }

  /// The bar's buttons for what is picked.
  List<List<_BarAction>> _barActions(L l) {
    final pts = [for (final q in _points) q.id];
    final edges = _edges;
    final notches = [for (final n in _notches) n.id];
    final guides = _guides;
    _BarAction act(
      String id,
      IconData icon,
      String name,
      String hint,
      VoidCallback onTap, {
      String? label,
    }) => (
      id: id,
      icon: icon,
      label: label ?? name,
      name: name,
      hint: hint,
      onTap: (_) => onTap(),
    );
    String kind(String k) => l.constraintKind(k);
    return switch ((pts.length, edges.length, notches.length, guides.length)) {
      (0, 0, 0, 0) => switch (_panel) {
        final panel? when _selection.length == 1 => _pieceActions(l, panel),
        _ when _selection.length > 1 => [
          [
            act(
              'delete',
              Icons.delete_outline_rounded,
              l.delete,
              l.barDeleteAllHint,
              _deleteSelected,
            ),
          ],
        ],
        _ => const [],
      },
      (1, 0, 0, 0) => () {
        final smooth = _constraints.any(
          (c) => c['kind'] == 'tangent' && c['point'] == pts[0],
        );
        return [
          [
            act(
              'fixed',
              Icons.lock_outline_rounded,
              kind('fixed'),
              l.barFixedHint,
              () => _constrain({'kind': 'fixed', 'point': pts[0]}),
            ),
          ],
          [
            act(
              'smooth',
              smooth ? Icons.change_history_rounded : Icons.gesture_rounded,
              smooth ? l.pointCorner : l.pointSmooth,
              smooth ? l.barCornerHint : l.barSmoothHint,
              () {
                _run({
                  'type': 'point.set_smooth',
                  'point': pts[0],
                  'smooth': !smooth,
                });
                _refresh();
              },
            ),
            act(
              'remove_point',
              Icons.remove_circle_outline_rounded,
              l.pointRemove,
              l.barRemovePointHint,
              label: l.delete,
              () {
                _run({'type': 'point.remove', 'point': pts[0]});
                _refresh();
              },
            ),
          ],
        ];
      }(),
      (2, 0, 0, 0) => [
        [
          for (final (along, name, icon, hint) in [
            ('horizontal', 'width', Icons.swap_horiz_rounded, l.barWidthHint),
            ('vertical', 'height', Icons.swap_vert_rounded, l.barHeightHint),
            (
              'direct',
              'distance',
              Icons.open_in_full_rounded,
              l.barDistanceHint,
            ),
          ])
            act(
              name,
              icon,
              kind(name),
              hint,
              () => _dimension({
                'kind': 'distance',
                'a': pts[0],
                'b': pts[1],
                'along': along,
              }),
            ),
        ],
        [
          act(
            'coincident',
            Icons.adjust_rounded,
            kind('coincident'),
            l.barJoinedHint,
            () => _constrain({'kind': 'coincident', 'a': pts[0], 'b': pts[1]}),
          ),
        ],
      ],
      (0, 1, 0, 0) => [
        [
          act(
            'length',
            Icons.straighten_rounded,
            kind('length'),
            l.barLengthHint,
            () => _dimension({'kind': 'length', 'edge': edges[0]}),
          ),
        ],
        [
          act(
            'horizontal',
            Icons.horizontal_rule_rounded,
            kind('horizontal'),
            l.barHorizontalHint,
            () => _constrain({'kind': 'horizontal', 'edge': edges[0]}),
          ),
          act(
            'vertical',
            Icons.height_rounded,
            kind('vertical'),
            l.barVerticalHint,
            () => _constrain({'kind': 'vertical', 'edge': edges[0]}),
          ),
        ],
        [
          act(
            'elastic',
            Icons.waves_rounded,
            l.barElastic,
            l.barElasticHint,
            () => _addElastic(edges[0]),
          ),
          if (_panel?['id'] case final String panel)
            act(
              'shirring',
              Icons.reorder_rounded,
              l.barShirring,
              l.barShirringHint,
              () => _shirr(panel, edges[0]),
            ),
        ],
      ],
      (0, 2, 0, 0) => [
        [
          act(
            'angle',
            Icons.architecture_rounded,
            kind('angle'),
            l.barAngleHint,
            () => _dimension({
              'kind': 'angle',
              'a': edges[0],
              'b': edges[1],
              'deg': 90,
            }),
          ),
        ],
        [
          act(
            'equal_length',
            Icons.drag_handle_rounded,
            kind('equal_length'),
            l.barSameLengthHint,
            () => _constrain({
              'kind': 'equal_length',
              'a': edges[0],
              'b': edges[1],
            }),
          ),
          act(
            'parallel',
            Icons.pause_rounded,
            kind('parallel'),
            l.barParallelHint,
            () =>
                _constrain({'kind': 'parallel', 'a': edges[0], 'b': edges[1]}),
          ),
          act(
            'perpendicular',
            Icons.square_foot_rounded,
            kind('perpendicular'),
            l.barSquareHint,
            () => _constrain({
              'kind': 'perpendicular',
              'a': edges[0],
              'b': edges[1],
            }),
          ),
        ],
      ],
      (0, final n, 0, 0) when n >= 3 => [
        [
          for (final (k, icon, hint) in [
            ('equal_length', Icons.drag_handle_rounded, l.barSameLengthHint),
            ('parallel', Icons.pause_rounded, l.barParallelHint),
          ])
            act(
              k,
              icon,
              kind(k),
              hint,
              () => _constrainAll(kind(k), [
                for (final e in edges.skip(1))
                  {'kind': k, 'a': edges[0], 'b': e},
              ]),
            ),
        ],
      ],
      (1, 1, 0, 0) => [
        [
          act(
            'on_edge',
            Icons.commit_rounded,
            kind('on_edge'),
            l.barOnEdgeHint,
            () => _constrain({
              'kind': 'on_edge',
              'point': pts[0],
              'edge': edges[0],
            }),
          ),
        ],
      ],
      (0, 0, 1, 0) => [
        [
          for (final (from, name, short, icon, hint) in [
            (
              'start',
              l.barNotchStart,
              l.barShortFromStart,
              Icons.first_page_rounded,
              l.barNotchStartHint,
            ),
            (
              'end',
              l.barNotchEnd,
              l.barShortFromEnd,
              Icons.last_page_rounded,
              l.barNotchEndHint,
            ),
          ])
            act(
              'notch_$from',
              icon,
              name,
              hint,
              label: short,
              () => _dimension({
                'kind': 'notch_distance',
                'notch': notches[0],
                'from': from,
              }),
            ),
        ],
        [
          act(
            'remove_notch',
            Icons.delete_outline_rounded,
            l.barRemoveNotch,
            l.barRemoveNotchHint,
            label: l.delete,
            () {
              _run({'type': 'notch.remove', 'notch': notches[0]});
              _refresh();
            },
          ),
        ],
      ],
      (1, 0, 0, 1) => [
        [
          act(
            'guide_distance',
            Icons.format_line_spacing_rounded,
            kind('guide_distance'),
            l.barGuideDistanceHint,
            label: kind('distance'),
            () => _dimension({
              'kind': 'guide_distance',
              'point': pts[0],
              'guide': guides[0],
            }),
          ),
        ],
        [
          act(
            'on_guide',
            Icons.commit_rounded,
            kind('on_guide'),
            l.barOnGuideHint,
            () => _constrain({
              'kind': 'on_guide',
              'point': pts[0],
              'guide': guides[0],
            }),
          ),
        ],
      ],
      (0, 0, 0, 1) => [
        [
          act(
            'level',
            Icons.horizontal_rule_rounded,
            l.barLevel,
            l.barLevelHint,
            () => _straightenGuide(guides[0], level: true),
          ),
          act(
            'upright',
            Icons.height_rounded,
            l.barUpright,
            l.barUprightHint,
            () => _straightenGuide(guides[0], level: false),
          ),
        ],
        [
          act(
            'remove_line',
            Icons.delete_outline_rounded,
            l.barRemoveLine,
            l.barRemoveLineHint,
            label: l.delete,
            () {
              _run({'type': 'guide.remove', 'guide': guides[0]});
              _refresh();
            },
          ),
        ],
      ],
      _ => const [],
    };
  }
}
