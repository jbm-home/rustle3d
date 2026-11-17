# 03 — Arrangement

Provenance: maintainer's functional brief (2026-11-14); general garment-making knowledge (where garment pieces sit on the body).

## User story
As a designer, I place panels around the avatar in a couple of clicks before
simulating, so that seams can close without tangling.

## Status (2026-11-15)

Implemented, simpler than below: each panel has a placement (angle around
the body's vertical axis, top height, offset; `panel.place`) and
`garment.arrange` ("Arrange around the body" in the inspector) places
every panel evenly in the order the seams join them, at the avatar's
waist. Panels start wrapped at the larger of the body's and the garment's
own radius. Since 2026-11-16 a placement can wrap a panel around the left
or right upper arm instead (sleeves; `panel.place` `around`), its top on
the arm's axis at the given height; since 2026-11-17 the axis turns at
the elbow to follow the forearm, so long sleeves start around it; `garment.arrange` keeps such
placements; or lay it flat, facing outward at a distance from the body's
axis (`flat`: a bag hanging from pins). The inspector sets a piece's
placement, and ⌘-drag in 3D moves a piece around the body. Schema v8
(M12): a placement may name the pattern abscissa that goes to its angle
(`centre_x_mm`, default the panel's horizontal centre); the pieces of a
cut panel keep the whole's, so they start where it was. Not yet: legs,
arrangement points shown in 3D.

## Behavior
- The avatar has **arrangement points**: front torso, back torso, left/right
  side, left/right arm (front/back), left/right leg (front/back), waist
  front/back. Each point has a position, an outward normal and a "wrap"
  surface: a cylinder fitted around the body part.
- Placing a panel on a point positions it in 3D, facing outward, at a
  configurable offset from the body, **wrapped** around the cylinder so that
  it is not flat in front of the body.
- **Automatic pre-positioning**: panels created from a template carry their
  arrangement point. For other panels, a heuristic proposes one from the
  panel's name and symmetry, and the user confirms.
- Manual adjustments: move along the surface, rotate around the normal,
  change the offset, flip the panel (inside/outside).
- Arrangement points come with the default avatars. For an imported avatar
  they are estimated from its bounding box and proportions, and can be edited.

## UX
- In 3D, arrangement points appear as discreet dots while the arrangement tool
  is active. Dragging a panel from the board onto a dot places it.
- A "Arrange all" button applies the suggested placement to every panel.

## Acceptance criteria
- With the skirt template, "Arrange all" places the front and back panels
  around the waist with no panel intersecting the avatar.
- Moving a panel along its cylinder keeps the offset from the body constant.
- Arrangement edits are undoable commands.
