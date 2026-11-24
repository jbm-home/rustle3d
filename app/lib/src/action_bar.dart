// The action bar above the 2D and 3D views (spec 30): what is picked on
// the board, and the constraints, dimensions and actions that fit it.

part of 'shell.dart';

/// One of the bar's buttons.
typedef _BarAction = ({
  String id,
  IconData icon,
  String name,
  String hint,
  VoidCallback onTap,
});

class _ActionBar extends StatelessWidget {
  const _ActionBar({required this.picked, required this.groups});

  /// What is picked ("2 corners · 1 edge"); empty: nothing.
  final String picked;

  /// The buttons, in groups set apart by a line.
  final List<List<_BarAction>> groups;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    Widget button(_BarAction a) => Padding(
      padding: const EdgeInsets.symmetric(horizontal: 1),
      child: Tooltip(
        richMessage: TextSpan(
          children: [
            TextSpan(
              text: '${a.name}\n',
              style: ts(fontWeight: FontWeight.w600),
            ),
            TextSpan(text: a.hint),
          ],
        ),
        child: _RailButton(
          key: ValueKey('bar_${a.id}'),
          icon: a.icon,
          label: a.name,
          active: false,
          grouped: false,
          onTap: a.onTap,
        ),
      ),
    );
    return Container(
      height: 50,
      margin: const EdgeInsets.fromLTRB(
        Space.s + 52 + Space.s,
        0,
        Space.m,
        Space.s,
      ),
      padding: const EdgeInsets.symmetric(horizontal: Space.m),
      decoration: BoxDecoration(
        color: p.surface,
        borderRadius: BorderRadius.circular(Tokens.radius(context)),
        border: Border.all(color: p.border),
      ),
      // Keys stay with the board (Delete, Enter, the tools).
      child: ExcludeFocus(
        child: Row(
          children: [
            Icon(Icons.ads_click_rounded, size: 16, color: p.textMuted),
            const SizedBox(width: Space.s),
            if (picked.isEmpty)
              Expanded(
                child: Text(
                  context.keys(l.barHint),
                  style: ts(fontSize: 12.5, color: p.textMuted),
                  overflow: TextOverflow.ellipsis,
                ),
              )
            else ...[
              Text(
                picked,
                style: ts(
                  fontSize: 12.5,
                  fontWeight: FontWeight.w600,
                  color: p.text,
                ),
              ),
              const SizedBox(width: Space.m),
              Expanded(
                child: SingleChildScrollView(
                  scrollDirection: Axis.horizontal,
                  child: Row(
                    children: [
                      for (final (i, group) in groups.indexed) ...[
                        if (i > 0)
                          Container(
                            width: 1,
                            height: 24,
                            margin: const EdgeInsets.symmetric(
                              horizontal: Space.s,
                            ),
                            color: p.border,
                          ),
                        for (final a in group) button(a),
                      ],
                    ],
                  ),
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }
}

extension _Bar on _ShellState {
  /// What is picked, in words ("2 corners · 1 edge"); empty: nothing.
  String _pickedText(L l) => [
    if (_points.isNotEmpty) l.barCorners(_points.length),
    if (_edges.isNotEmpty) l.barEdges(_edges.length),
    if (_notches.isNotEmpty) l.barNotches(_notches.length),
    if (_guides.isNotEmpty) l.barLines(_guides.length),
  ].join(' · ');

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
      VoidCallback onTap,
    ) => (id: id, icon: icon, name: name, hint: hint, onTap: onTap);
    String kind(String k) => l.constraintKind(k);
    return switch ((pts.length, edges.length, notches.length, guides.length)) {
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
          for (final (from, name, icon, hint) in [
            (
              'start',
              l.barNotchStart,
              Icons.first_page_rounded,
              l.barNotchStartHint,
            ),
            ('end', l.barNotchEnd, Icons.last_page_rounded, l.barNotchEndHint),
          ])
            act(
              'notch_$from',
              icon,
              name,
              hint,
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
