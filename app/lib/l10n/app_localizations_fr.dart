// ignore: unused_import
import 'package:intl/intl.dart' as intl;

import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for French (`fr`).
class LFr extends L {
  LFr([String locale = 'fr']) : super(locale);

  @override
  String get languageTooltip => 'Langue';

  @override
  String get themeLight => 'Thème clair';

  @override
  String get themeDark => 'Thème sombre';

  @override
  String get cancel => 'Annuler';

  @override
  String get create => 'Créer';

  @override
  String get save => 'Enregistrer';

  @override
  String get apply => 'Appliquer';

  @override
  String get delete => 'Supprimer';

  @override
  String get next => 'Suivant';

  @override
  String get skip => 'Passer';

  @override
  String get start => 'Commencer';

  @override
  String get gotIt => 'Compris';

  @override
  String get cm => 'cm';

  @override
  String get keyCtrl => 'Ctrl';

  @override
  String get keyAlt => 'Alt';

  @override
  String get keyShift => 'Maj';

  @override
  String get keyDelete => 'Suppr';

  @override
  String get startTitle => 'Que voulez-vous créer ?';

  @override
  String get templateTShirt => 'T-shirt';

  @override
  String get templateTShirtBlurb =>
      'Devant, dos et manches courtes, en jersey.';

  @override
  String get templateShirt => 'Chemise';

  @override
  String get templateShirtBlurb =>
      'Manches longues à poignets et col officier.';

  @override
  String get templateTrousers => 'Pantalon';

  @override
  String get templateTrousersBlurb =>
      'Jambes droites, de la taille à la cheville.';

  @override
  String get templateHoodie => 'Sweat à capuche';

  @override
  String get templateHoodieBlurb =>
      'En jersey, manches longues, une capuche, froncé aux poignets et en bas.';

  @override
  String get templateJoggers => 'Jogging';

  @override
  String get templateJoggersBlurb =>
      'En jersey, à enfiler : taille et chevilles élastiques.';

  @override
  String get templateDress => 'Robe';

  @override
  String get templateDressBlurb =>
      'Sans manches, légèrement cintrée, aux genoux.';

  @override
  String get templateStraightSkirt => 'Jupe droite';

  @override
  String get templateStraightSkirtBlurb =>
      'Près des hanches, de la taille aux genoux.';

  @override
  String get templateALineSkirt => 'Jupe trapèze';

  @override
  String get templateALineSkirtBlurb =>
      'S’évase doucement des hanches à l’ourlet.';

  @override
  String get templateToteBag => 'Sac cabas';

  @override
  String get templateToteBagBlurb =>
      'Deux pièces et quatre coutures : le plus simple pour commencer.';

  @override
  String get sectionNew => 'Nouveau';

  @override
  String get openFromDisk => 'Ouvrir…';

  @override
  String get open => 'Ouvrir';

  @override
  String allProjectsCount(int count) {
    return 'Tous les projets ($count)';
  }

  @override
  String get projectsFolder => 'Dossier des projets';

  @override
  String get importProjectZip => 'Importer un .zip…';

  @override
  String get importProjectZipTip =>
      'Ajouter un projet reçu sous forme d’archive .zip';

  @override
  String get projectZipFiles => 'Archives de projet';

  @override
  String get projectOptions => 'Options du projet';

  @override
  String get renameProject => 'Renommer…';

  @override
  String get exportProjectZip => 'Exporter en .zip…';

  @override
  String get deleteProject => 'Supprimer…';

  @override
  String get renameProjectTitle => 'Renommer le projet';

  @override
  String get rename => 'Renommer';

  @override
  String deleteProjectTitle(String name) {
    return 'Supprimer « $name » ?';
  }

  @override
  String get deleteProjectBody =>
      'Le dossier du projet et tout son contenu (patron, corps, exports) seront supprimés de cet ordinateur. Cette action est irréversible.';

  @override
  String get deleteProjectConfirm =>
      'Je comprends que ce projet sera supprimé définitivement';

  @override
  String projectDeleted(String name) {
    return '« $name » supprimé.';
  }

  @override
  String projectExported(String name, String file) {
    return '« $name » exporté dans $file.';
  }

  @override
  String appVersion(String version) {
    return 'Version $version';
  }

  @override
  String get updatesLink => 'Mises à jour et actualités sur rustle3d.com';

  @override
  String get supportLink => 'Soutenir Rustle3D';

  @override
  String get supportTitle => 'Rustle3D est libre et gratuit';

  @override
  String get supportText =>
      'S’il vous est utile, un don aide à couvrir ses frais : hébergement, matériel de test et temps de développement. Tout reste gratuit dans tous les cas.';

  @override
  String get supportDonate => 'Faire un don';

  @override
  String get supportLater => 'Pas maintenant';

  @override
  String get supportTurnOff => 'Ne plus afficher';

  @override
  String updateTitle(String version) {
    return 'Rustle3D $version est disponible';
  }

  @override
  String get updateText =>
      'Téléchargez-la sur rustle3d.com et installez-la par-dessus cette version : vos projets sont conservés.';

  @override
  String get updateDownload => 'Télécharger';

  @override
  String get updateSkip => 'Ignorer cette version';

  @override
  String get updatesTitle => 'Mises à jour';

  @override
  String get updateAuto => 'Rechercher les mises à jour une fois par jour';

  @override
  String get updateAutoHint =>
      'L’application demande à rustle3d.com le numéro de la dernière version, en n’envoyant que sa propre version et son système. Elle ne télécharge ni n’installe jamais rien d’elle-même.';

  @override
  String get updateCheckNow => 'Vérifier maintenant';

  @override
  String get updateLatest => 'Vous avez la dernière version.';

  @override
  String get updateFailed =>
      'Impossible de joindre rustle3d.com. Vérifiez votre connexion et réessayez.';

  @override
  String get updateCheck => 'Rechercher les mises à jour';

  @override
  String cannotOpen(String target) {
    return 'Impossible d’ouvrir $target.';
  }

  @override
  String get blankProject => 'Projet vierge';

  @override
  String get blankProjectBlurb => 'Dessinez vos propres pièces.';

  @override
  String get untitled => 'Sans titre';

  @override
  String get recent => 'Récents';

  @override
  String get recentEmpty =>
      'Vos projets apparaîtront ici. Ils sont enregistrés au fil du travail, dans Documents › Rustle3D.';

  @override
  String get agoJustNow => 'à l’instant';

  @override
  String agoMinutes(int count) {
    return 'il y a $count min';
  }

  @override
  String agoHours(int count) {
    return 'il y a $count h';
  }

  @override
  String get agoYesterday => 'hier';

  @override
  String agoDays(int count) {
    return 'il y a $count jours';
  }

  @override
  String newGarmentTitle(String garment) {
    return 'Nouveau : $garment';
  }

  @override
  String get fieldName => 'Nom';

  @override
  String get fieldBody => 'Corps';

  @override
  String sizeMustBePositive(String size) {
    return '$size doit être un nombre de cm positif.';
  }

  @override
  String get sizeChest => 'Tour de poitrine';

  @override
  String get sizeChestHint =>
      'Autour de la partie la plus forte de la poitrine';

  @override
  String get sizeWaist => 'Tour de taille';

  @override
  String get sizeWaistHint => 'Autour de la partie la plus fine';

  @override
  String get sizeHips => 'Tour de hanches';

  @override
  String get sizeHipsHint => 'Autour de la partie la plus forte';

  @override
  String get sizeLength => 'Longueur';

  @override
  String get sizeLengthHint =>
      'Du cou à l’ourlet pour un haut, de la taille à l’ourlet pour une jupe, hauteur d’un sac';

  @override
  String get sizeSleeve => 'Manche';

  @override
  String get sizeSleeveHint => 'Depuis le haut de l’épaule';

  @override
  String get sizeWidth => 'Largeur';

  @override
  String get sizeWidthHint => 'Largeur du sac';

  @override
  String get sizeEase => 'Aisance';

  @override
  String get sizeEaseHint => 'Ampleur ajoutée au corps';

  @override
  String get pieceFront => 'Devant';

  @override
  String get pieceBack => 'Dos';

  @override
  String get pieceBackLeft => 'Dos gauche';

  @override
  String get pieceBackRight => 'Dos droit';

  @override
  String get pieceSleeveLeft => 'Manche gauche';

  @override
  String get pieceSleeveRight => 'Manche droite';

  @override
  String pieceNew(int number) {
    return 'Pièce $number';
  }

  @override
  String pieceMirror(String name) {
    return '$name (symétrique)';
  }

  @override
  String get body => 'Corps';

  @override
  String get bodySubtitle =>
      'Le corps 3D est ajusté à ces mesures. Partez d’une taille, puis ajustez.';

  @override
  String get bodyPreset => 'Modèle de corps';

  @override
  String presetWomen(String size) {
    return 'Femme $size';
  }

  @override
  String presetMen(String size) {
    return 'Homme $size';
  }

  @override
  String get bodyFeminine => 'Corps féminin';

  @override
  String get bodyMasculine => 'Corps masculin';

  @override
  String get measHeight => 'Stature';

  @override
  String get measBust => 'Tour de poitrine';

  @override
  String measBetween(String measure, int min, int max) {
    return '$measure doit être entre $min et $max cm.';
  }

  @override
  String get presetName => 'Nom du modèle';

  @override
  String get saveAsPreset => 'Enregistrer comme modèle';

  @override
  String get deletePreset => 'Supprimer le modèle';

  @override
  String fittedBody(String height, String bust, String waist, String hips) {
    return 'Corps ajusté : hauteur $height · poitrine $bust · taille $waist · hanches $hips';
  }

  @override
  String get bodyMore => 'Plus de mesures';

  @override
  String get bodyMoreHint =>
      'Estimées à partir des quatre ci-dessus tant que vous ne saisissez pas les vôtres.';

  @override
  String get bodyTorso => 'Torse';

  @override
  String get bodyArms => 'Bras';

  @override
  String get bodyLegs => 'Jambes';

  @override
  String get measEstimated => 'estimée';

  @override
  String get measReset => 'Revenir à l\'estimation';

  @override
  String get measNeck => 'Tour de cou';

  @override
  String get measNeckHint => 'Autour de la base du cou';

  @override
  String get measShoulderWidth => 'Carrure dos';

  @override
  String get measShoulderWidthHint =>
      'En ligne droite dans le dos, d\'une pointe d\'épaule à l\'autre';

  @override
  String get measShoulderLength => 'Longueur d\'épaule';

  @override
  String get measShoulderLengthHint =>
      'Du côté du cou à la pointe de l\'épaule';

  @override
  String get measBackWaistLength => 'Longueur taille dos';

  @override
  String get measBackWaistLengthHint =>
      'De la nuque à la taille, le long de la colonne';

  @override
  String get measFrontWaistLength => 'Longueur taille devant';

  @override
  String get measFrontWaistLengthHint =>
      'Du côté du cou à la taille, en passant par la pointe de poitrine';

  @override
  String get measWaistHeight => 'Hauteur de taille';

  @override
  String get measWaistHeightHint => 'De la taille au sol';

  @override
  String get measHipHeight => 'Hauteur de hanches';

  @override
  String get measHipHeightHint => 'Du plus fort des hanches au sol';

  @override
  String get measCrotchDepth => 'Montant';

  @override
  String get measCrotchDepthHint =>
      'De la taille à l\'entrejambe (assis : jusqu\'à la chaise)';

  @override
  String get measArmLength => 'Longueur de bras';

  @override
  String get measArmLengthHint =>
      'De la pointe de l\'épaule à l\'os du poignet, coude plié';

  @override
  String get measUpperArm => 'Tour de bras';

  @override
  String get measUpperArmHint => 'Autour du haut du bras, au plus fort';

  @override
  String get measWrist => 'Tour de poignet';

  @override
  String get measWristHint => 'Autour du poignet';

  @override
  String get measInsideLeg => 'Entrejambe';

  @override
  String get measInsideLegHint =>
      'De l\'entrejambe au sol, à l\'intérieur de la jambe';

  @override
  String get measThigh => 'Tour de cuisse';

  @override
  String get measThighHint =>
      'Autour de la cuisse au plus fort, juste sous l\'entrejambe';

  @override
  String get measKnee => 'Tour de genou';

  @override
  String get measKneeHint => 'Autour du genou';

  @override
  String get measAnkle => 'Tour de cheville';

  @override
  String get measAnkleHint => 'Juste au-dessus de l\'os de la cheville';

  @override
  String measAvatarReaches(String value) {
    return 'L\'avatar mesure $value : son corps ne peut pas aller plus loin.';
  }

  @override
  String get allProjects => 'Tous les projets';

  @override
  String get export => 'Exporter';

  @override
  String get importPattern => 'Importer';

  @override
  String get importTip =>
      'Importer un patron dessiné ailleurs (DXF-AAMA, DXF, SVG, PDF, Seamly2D ou Valentina), ou un fichier de mensurations Seamly2D ou Valentina';

  @override
  String importUnsupported(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          '$count types d’étapes de tracé ne sont pas encore pris en charge : les pièces qui les utilisent ont été laissées de côté.',
      one: '1 type d’étape de tracé n’est pas encore pris en charge : les pièces qui l’utilisent ont été laissées de côté.',
    );
    return '$_temp0';
  }

  @override
  String get measurementsImported =>
      'L’avatar prend les mensurations du fichier.';

  @override
  String garmentSource(String file) {
    return 'Tracé depuis $file';
  }

  @override
  String get garmentRedraft => 'Retracer pour l’avatar';

  @override
  String get garmentRedraftTip =>
      'Relancer le patron avec les mensurations de l’avatar ; vos modifications des pièces sont conservées';

  @override
  String get busyRedraft => 'Nouveau tracé du patron';

  @override
  String get importFiles => 'Patrons';

  @override
  String imported(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count pièces importées',
      one: '1 pièce importée',
    );
    return '$_temp0.';
  }

  @override
  String importIgnored(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count tracés ne faisaient partie d’aucune pièce.',
      one: '1 tracé ne faisait partie d’aucune pièce.',
    );
    return '$_temp0';
  }

  @override
  String get importUnitsTitle => 'Dans quelle unité ce patron est-il dessiné ?';

  @override
  String get importUnitsBody =>
      'Le fichier n’indique pas son échelle : les pièces n’ont peut-être pas la bonne taille. Choisissez l’unité du dessin, ou gardez-les telles quelles.';

  @override
  String get importKeep => 'Garder tel quel';

  @override
  String get unitMm => 'Millimètres';

  @override
  String get unitCm => 'Centimètres';

  @override
  String get unitIn => 'Pouces';

  @override
  String get unitPt => 'Points (1/72 de pouce)';

  @override
  String get unitPx => 'Pixels (1/96 de pouce)';

  @override
  String get agents => 'Agents';

  @override
  String get agentsWaiting => 'Agents : en attente';

  @override
  String get agentConnected => 'Agent connecté';

  @override
  String get agentsTip =>
      'Agents IA : laisser un agent modifier ce projet en direct';

  @override
  String get allowAgents => 'Autoriser les agents IA';

  @override
  String get agentsTitle => 'Connecter un agent IA';

  @override
  String get agentsIntro =>
      'Un agent IA (un assistant qui utilise des outils) peut modifier ce projet en direct : tracer des pièces, coudre, simuler, regarder le résultat. Il dialogue avec Rustle3D par MCP ; les étapes ci-dessous prennent une minute, une seule fois.';

  @override
  String get agentsStepAllow => 'Laisser entrer les agents';

  @override
  String get agentsStepAdd => 'Ajouter Rustle3D à votre agent';

  @override
  String get agentsAddHint =>
      'Ajoutez ce serveur aux serveurs MCP de votre agent, puis relancez l’agent. Il lance r3d, l’outil en ligne de commande installé avec l’app.';

  @override
  String get agentsJsonHint =>
      'La plupart des assistants de bureau lisent ceci dans leur fichier de réglages MCP ; fusionnez-le dans son « mcpServers ».';

  @override
  String get agentsExtensionHint =>
      'Un fichier d’extension pour Claude Desktop et les autres assistants qui installent les paquets MCP (.mcpb) : enregistrez-le, puis ouvrez-le, ou déposez-le dans Réglages > Extensions de l’assistant. Il lance r3d depuis le dossier de l’application, il reste donc valable après une mise à jour ; si vous déplacez l’application, enregistrez-le à nouveau.';

  @override
  String get agentsSaveExtension => 'Enregistrer rustle3d.mcpb…';

  @override
  String agentsExtensionSaved(String path) {
    return 'Enregistré : $path. Ouvrez-le pour l’installer dans votre assistant.';
  }

  @override
  String get agentsCopy => 'Copier';

  @override
  String get agentsNoR3d =>
      'r3d est introuvable à côté de l’app (une version compilée depuis les sources ?) : compilez-le avec cargo build -p r3d-cli et mettez son chemin complet à la place de r3d.';

  @override
  String get agentsStepAsk => 'Demandez à votre agent';

  @override
  String get agentsAskHint =>
      'Par exemple : « Regarde mon projet dans Rustle3D et rallonge les manches de 3 cm. » Chaque modification s’affiche ici aussitôt et peut être annulée.';

  @override
  String get agentsStateOff => 'Les agents ne sont pas autorisés';

  @override
  String get agentsStateWaiting => 'En attente d’un agent…';

  @override
  String get disconnectAgent => 'Déconnecter l’agent';

  @override
  String get exportSvg => 'Patron, SVG à l’échelle 1';

  @override
  String get exportPdfA4 => 'Patron, PDF en pages A4';

  @override
  String get exportPdfSingle => 'Patron, PDF sur une seule page';

  @override
  String get exportGlb => 'Vêtement en 3D, glTF (.glb)';

  @override
  String get exportGlbAvatar => 'Vêtement et corps en 3D, glTF (.glb)';

  @override
  String get exportObj => 'Vêtement en 3D, OBJ';

  @override
  String get exportDxf => 'Patron, DXF-AAMA (logiciels de patronage)';

  @override
  String exported(String file) {
    return 'Exporté : $file.';
  }

  @override
  String get learn => 'Apprendre';

  @override
  String get undoTip => 'Annuler  ⌘Z';

  @override
  String get redoTip => 'Rétablir  ⇧⌘Z';

  @override
  String get tourPatternTitle => 'Votre patron';

  @override
  String get tourPatternText =>
      'Les pièces à plat, en taille réelle. Faites glisser un coin ou un bord pour les modifier ; **R** ajoute une pièce.';

  @override
  String get tourGarmentTitle => 'Le vêtement sur le corps';

  @override
  String get tourGarmentText =>
      '**Simuler** le fait tomber sur le corps. Faites glisser pour tourner autour ; **⇧-glisser** un ourlet ou une couture modifie le patron depuis ici.';

  @override
  String get tourHelpTitle => 'L’aide est toujours là';

  @override
  String get tourHelpText =>
      '**Apprendre** propose de courts tutoriels et montre où se trouvent les choses ; **Corps** change les mesures. **⌘Z** annule tout.';

  @override
  String get toolSelect => 'Sélection';

  @override
  String get toolSelectHint =>
      'Cliquez une pièce ; faites-la glisser pour la déplacer, les coins de son cadre pour la mettre à l’échelle, la poignée ronde au-dessus pour la faire pivoter (⇧ : par pas de 15°, ou dans les deux sens à la fois) ; faites glisser un coin ou un bord pour la modifier';

  @override
  String get toolRectangle => 'Rectangle';

  @override
  String get toolRectangleHint => 'Ajouter une pièce rectangulaire';

  @override
  String get toolPolygon => 'Polygone';

  @override
  String get toolPolygonHint => 'Cliquez les coins d’une nouvelle pièce';

  @override
  String get toolCurve => 'Courbe';

  @override
  String get toolCurveHint =>
      'Tracez une pièce : cliquez pour un coin, glissez pour un point arrondi';

  @override
  String get toolEditCurve => 'Modifier les courbes';

  @override
  String get toolEditCurveHint =>
      'Courber un bord, le redresser ou y ajouter un point';

  @override
  String get toolBend => 'Courber';

  @override
  String get toolBendHint =>
      'Cliquez un bord, puis faites glisser ses poignées pour le courber';

  @override
  String get toolStraighten => 'Redresser';

  @override
  String get toolStraightenHint =>
      'Cliquez un bord courbe pour le rendre droit';

  @override
  String get toolAddPoint => 'Ajouter un point';

  @override
  String get toolAddPointHint => 'Cliquez un bord là où va le nouveau point';

  @override
  String get toolCut => 'Couper';

  @override
  String get toolCutHint =>
      'Tracez une ligne à travers une pièce : elle devient deux pièces cousues';

  @override
  String get toolDraw => 'Dessiner';

  @override
  String get toolDrawHint =>
      'Faites glisser sur une pièce, dans le patron ou sur le vêtement 3D : la ligne va sur le patron';

  @override
  String get toolNotch => 'Cran';

  @override
  String get toolNotchHint => 'Cliquez un bord à l’endroit du cran';

  @override
  String get toolSew => 'Coudre';

  @override
  String get toolSewHint =>
      'Cliquez deux bords ou lignes sur le patron ; glissez le long d’un bord pour n’en coudre qu’une partie';

  @override
  String get viewPattern => 'Patron';

  @override
  String get view3d => '3D';

  @override
  String get hintPolygon =>
      'Polygone : cliquez les coins ; cliquez le premier ou Entrée pour fermer · Échap annule · V pour arrêter';

  @override
  String get hintCurve =>
      'Courbe : cliquez pour un coin, glissez pour un point arrondi ; cliquez le premier point ou Entrée pour fermer · Échap annule · V pour arrêter';

  @override
  String get hintBend =>
      'Courber : cliquez un bord, puis faites glisser ses poignées · V pour arrêter';

  @override
  String get hintStraighten =>
      'Redresser : cliquez un bord courbe · V pour arrêter';

  @override
  String get hintAddPoint =>
      'Ajouter un point : cliquez un bord là où va le point · V pour arrêter';

  @override
  String get hintCut =>
      'Couper : cliquez à travers une pièce, glissez pour une courbe, de l’extérieur à l’extérieur ; Entrée coupe · Échap annule · V pour arrêter';

  @override
  String get hintNotch =>
      'Cran : cliquez un bord à l’endroit du cran · V pour arrêter';

  @override
  String get toastPieceAdded => 'Pièce ajoutée.';

  @override
  String get toastNotchAdded => 'Cran ajouté.';

  @override
  String get toastCut => 'Coupée en deux pièces, cousues ensemble.';

  @override
  String get toastPointAdded => 'Point ajouté.';

  @override
  String get distance => 'Distance';

  @override
  String get pointPicked =>
      'Coin choisi. ⌘-cliquez un autre coin pour mesurer la distance entre eux.';

  @override
  String get pointsPicked =>
      'Entre les deux coins. Le second se déplace le long de la ligne.';

  @override
  String get straighten => 'Redresser';

  @override
  String get hintSewFirst =>
      'Coudre : cliquez un bord ou une ligne, ou glissez le long pour n’en coudre qu’une partie · ⌘-cliquez des bords qui se suivent pour les coudre comme un seul côté · V pour arrêter';

  @override
  String get hintSewSecond =>
      'Coudre : maintenant le côté à y coudre : cliquez-le, glissez le long d’une partie ou ⌘-cliquez plusieurs bords · Échap pour recommencer';

  @override
  String get hintSewRun =>
      'Coudre : ⌘-cliquez les bords suivants, puis cliquez le dernier ou appuyez sur Entrée · Échap pour recommencer';

  @override
  String get hintDraw =>
      'Dessin : faites glisser sur le vêtement 3D ; la ligne va sur le patron · V pour arrêter';

  @override
  String get hintIdle =>
      'Patron : glissez un coin ou un bord, cliquez un bord pour sa longueur · 3D : glissez pour tirer le tissu · ⇧-glisser un ourlet ou une couture · ⌘-glisser pour déplacer une pièce · ⌥-clic pour épingler';

  @override
  String get hintSelection =>
      'Réglez la sélection dans l’inspecteur · ⌫ supprime · ⌘Z annule';

  @override
  String statsSim(int points, String backend, String stepMs, String time) {
    return 'Tissu $points points · pas $backend $stepMs ms · $time s · ';
  }

  @override
  String statsRender(String boardMs, String garmentMs, String fps) {
    return 'Rendu Rust · patron $boardMs ms · 3D $garmentMs ms · $fps i/s';
  }

  @override
  String get toastSewn =>
      'Cousu. Inversez-le dans l’inspecteur s’il paraît vrillé.';

  @override
  String get toastPinned => 'Épinglé. Retirez l’épingle dans l’inspecteur.';

  @override
  String get toastLineAdded => 'Ligne ajoutée au patron.';

  @override
  String deleteTitle(String name) {
    return 'Supprimer « $name » ?';
  }

  @override
  String get deleteUndo => 'Vous pourrez annuler avec ⌘Z.';

  @override
  String get camFront => 'Face';

  @override
  String get camSide => 'Profil';

  @override
  String get camBack => 'Dos';

  @override
  String get camFrontTip => 'Vue de face  1';

  @override
  String get camSideTip => 'Vue de profil  2';

  @override
  String get camBackTip => 'Vue de dos  3';

  @override
  String get camThreeQuarterTip => 'Vue de trois quarts  4';

  @override
  String get simulate => 'Simuler';

  @override
  String get pause => 'Pause';

  @override
  String get simulateTip => 'Faire tomber le vêtement  Espace';

  @override
  String get pauseTip => 'Pause  Espace';

  @override
  String get restartTip => 'Recommencer depuis le patron';

  @override
  String get motionTip => 'Faire bouger l\'avatar';

  @override
  String get motionStill => 'Immobile';

  @override
  String get motionWalk => 'Marcher';

  @override
  String get motionIdle => 'Attendre';

  @override
  String get motionTurn => 'Se tourner';

  @override
  String get motionRaiseArms => 'Lever les bras';

  @override
  String get maximizeViewTip => 'Agrandir à la place des deux vues  M';

  @override
  String get restoreViewsTip => 'Revenir aux deux vues  M';

  @override
  String get inspector => 'Inspecteur';

  @override
  String get selectPanel => 'Sélectionnez une pièce';

  @override
  String selectPanelHint(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count pièces',
      one: '1 pièce',
      zero: 'Aucune pièce',
    );
    return 'Cliquez une pièce sur le patron ou le vêtement\npour la régler ici. $_temp0 dans ce projet.';
  }

  @override
  String get arrange => 'Disposer autour du corps';

  @override
  String get arrangeHint =>
      'Place chaque pièce autour du corps dans l’ordre des coutures, à la taille.';

  @override
  String get pins => 'Épingles';

  @override
  String get pinsEmpty =>
      '⌥-clic sur le tissu en 3D pour le maintenir en place.';

  @override
  String get pinsAdd => '⌥-clic sur le tissu en 3D pour en ajouter une.';

  @override
  String get removePin => 'Retirer cette épingle';

  @override
  String get wholeGarment => 'Vêtement entier';

  @override
  String piecesSelected(int count) {
    return '$count pièces sélectionnées. Cliquez une seule pièce sur le patron pour la modifier.';
  }

  @override
  String get piece => 'Pièce';

  @override
  String get renamePiece => 'Renommer la pièce (ou double-cliquer sur son nom)';

  @override
  String get renamePieceOk => 'Renommer (Entrée)';

  @override
  String get mirrorCopy => 'Copie symétrique — suit sa source';

  @override
  String get unlink => 'Délier';

  @override
  String get area => 'Surface';

  @override
  String get perimeter => 'Périmètre';

  @override
  String get seamAllowance => 'Valeur de couture';

  @override
  String get grainline => 'Droit-fil';

  @override
  String get mirror => 'Symétrique';

  @override
  String get onTheBody => 'Sur le corps';

  @override
  String get notSimulated =>
      'Pas encore simulée : placez-la pour la voir en 3D.';

  @override
  String get placeOnBody => 'Placer sur le corps';

  @override
  String get aroundBody => 'Corps';

  @override
  String get aroundLeftArm => 'Bras gauche';

  @override
  String get aroundRightArm => 'Bras droit';

  @override
  String get aroundLeftLeg => 'Jambe gauche';

  @override
  String get aroundRightLeg => 'Jambe droite';

  @override
  String get aroundFlat => 'À plat';

  @override
  String get aroundAngle => 'Autour (0 = devant)';

  @override
  String get aroundArmAngle => 'Autour du bras (0 = extérieur)';

  @override
  String get topHeight => 'Hauteur du haut';

  @override
  String get fabric => 'Tissu';

  @override
  String get garments => 'Vêtements';

  @override
  String get garmentsHint =>
      'Portés du corps vers l’extérieur : un vêtement se drape par-dessus ceux qui le précèdent dans la liste.';

  @override
  String get garmentNextToBody => 'Contre le corps';

  @override
  String get garmentOver => 'Porté par-dessus le précédent';

  @override
  String get garmentMoveIn => 'Le porter dessous';

  @override
  String get garmentMoveOut => 'Le porter dessus';

  @override
  String get garmentTakeOut => 'Le sortir du drapé';

  @override
  String get garmentPutBack => 'Le remettre dans le drapé';

  @override
  String get garmentOutOfDrape => 'Hors du drapé';

  @override
  String get garmentRemove => 'Supprimer le vêtement et ses pièces';

  @override
  String get addGarment => 'Ajouter un vêtement';

  @override
  String garmentNewName(int number) {
    return 'Vêtement $number';
  }

  @override
  String get pieceGarment => 'Vêtement';

  @override
  String get fabricCotton => 'Coton';

  @override
  String get fabricCottonBlurb => 'Net et respirant';

  @override
  String get fabricDenim => 'Denim';

  @override
  String get fabricDenimBlurb => 'Lourd, garde sa forme';

  @override
  String get fabricJersey => 'Jersey';

  @override
  String get fabricJerseyBlurb => 'Maille extensible';

  @override
  String get fabricSilk => 'Soie';

  @override
  String get fabricSilkBlurb => 'Légère et fluide';

  @override
  String get fabricWool => 'Laine';

  @override
  String get fabricWoolBlurb => 'Douce, avec de la tenue';

  @override
  String edgeLength(String length) {
    return 'Bord · $length cm';
  }

  @override
  String get edgeOpen =>
      'Bord libre (un ourlet ou une ouverture). En 3D, ⇧-glissez-le pour rallonger le vêtement.';

  @override
  String get edgeLengthLabel => 'Longueur';

  @override
  String edgeSewn(String piece, String mismatch) {
    return 'Cousu à $piece · $mismatch mm d’écart. Sa longueur change des deux côtés. En 3D, ⇧-glissez-le pour le reprendre.';
  }

  @override
  String get matchOtherSide => 'Mettre l’autre côté à cette longueur';

  @override
  String get gathered => 'Froncée';

  @override
  String gatheredHint(String ratio) {
    return 'Le côté le plus long est froncé régulièrement sur le plus court ($ratio : 1).';
  }

  @override
  String get flipSeam => 'Inverser la couture';

  @override
  String get unpick => 'Découdre';

  @override
  String changeTakenIn(String amount) {
    return 'Repris de $amount mm';
  }

  @override
  String changeLetOut(String amount) {
    return 'Relâché de $amount mm';
  }

  @override
  String changeLonger(String amount) {
    return 'Rallongé de $amount mm';
  }

  @override
  String changeMoved(String amount) {
    return 'Déplacé de $amount mm';
  }

  @override
  String changeShorter(String amount) {
    return 'Raccourci de $amount mm';
  }

  @override
  String get learnIntro =>
      'De courts tutoriels. « Montrer » indique où cela se passe ; votre projet n’est pas modifié.';

  @override
  String get showMe => 'Montrer';

  @override
  String get learnDrape => 'Faire tomber le vêtement';

  @override
  String get learnDrapeHow =>
      'Appuyez sur **Simuler** (**Espace**) : les pièces ferment leurs coutures et tombent sur le corps.';

  @override
  String get learnFabric => 'Essayer un autre tissu';

  @override
  String get learnFabricHow =>
      'Sans rien sélectionner, choisissez un échantillon sous **Tissu**. Le tissu change le tombé du vêtement.';

  @override
  String get learnLength => 'Changer une longueur';

  @override
  String get learnLengthHow =>
      'Cliquez un bord (une couture de côté pour rallonger, un ourlet pour élargir), puis tapez sa **Longueur** ou faites glisser le curseur. Ou maintenez **⇧** et faites glisser l’ourlet sur le vêtement.';

  @override
  String get learnFit => 'Reprendre une couture';

  @override
  String get learnFitHow =>
      'Maintenez **⇧** et faites glisser une couture sur le vêtement : vers le corps la reprend, vers l’extérieur la relâche.';

  @override
  String get learnSew => 'Coudre deux pièces';

  @override
  String get learnSewHow =>
      'Avec **Coudre** (**S**), cliquez un bord sur le patron, puis le bord auquel le coudre.';

  @override
  String get learnDraw => 'Tracer une ligne de style';

  @override
  String get learnDrawHow =>
      'Avec **Dessiner sur le vêtement** (**D**), faites glisser sur le vêtement 3D : la ligne arrive sur le patron.';

  @override
  String get learnMove => 'Déplacer une pièce autour du corps';

  @override
  String get learnMoveHow =>
      'Maintenez **⌘** et faites glisser une pièce sur le vêtement, ou réglez-la dans l’inspecteur.';

  @override
  String get learnPin => 'Tirer et épingler le tissu';

  @override
  String get learnPinHow =>
      'Faites glisser le tissu en 3D pour le tirer. **⌥-clic** l’épingle là où il est.';

  @override
  String get learnLook => 'Regarder autour';

  @override
  String get learnLookHow =>
      'Faites glisser pour tourner autour du corps ; molette ou pincement pour zoomer. Touches **1**–**4** : face, profil, dos, trois quarts.';

  @override
  String get learnBody => 'Habiller un autre corps';

  @override
  String get learnBodyHow =>
      'Ouvrez **Corps** : choisissez une taille ou entrez des mesures, et le vêtement tombe dessus.';

  @override
  String get learnDrapeStop => 'Simuler';

  @override
  String get learnDrapeStopText =>
      'Appuyez sur **Simuler** (**Espace**) pour faire tomber le vêtement, à nouveau pour mettre en pause. La flèche à côté recommence depuis le patron.';

  @override
  String get learnFabricStop => 'Tissu';

  @override
  String get learnFabricStopText =>
      'Sans rien sélectionner, le panneau de droite montre le vêtement : choisissez un échantillon sous **Tissu**. Le tombé suit.';

  @override
  String get learnPickEdge => 'Choisir un bord';

  @override
  String get learnPickEdgeText =>
      'Cliquez un bord sur le patron, ou sur le vêtement.';

  @override
  String get learnEdgeLength => 'Sa longueur';

  @override
  String get learnEdgeLengthText =>
      'Tapez la nouvelle **Longueur** puis **Entrée**, ou faites glisser le curseur. L’autre côté d’un bord cousu suit, les coutures restent alignées.';

  @override
  String get learnDragHem => 'Ou faites-le glisser';

  @override
  String get learnDragHemText =>
      'Maintenez **⇧** et faites glisser l’ourlet sur le vêtement : vers le bas rallonge, vers le haut raccourcit.';

  @override
  String get learnDragSeam => 'Ou faites-la glisser';

  @override
  String get learnDragSeamText =>
      'Maintenez **⇧** et faites glisser une couture sur le vêtement : vers le corps la reprend, vers l’extérieur la relâche. Près d’une extrémité, seule cette extrémité bouge.';

  @override
  String get learnSewTool => 'Outil Coudre';

  @override
  String get learnSewToolText =>
      'Choisissez **Coudre** ici, ou appuyez sur **S**.';

  @override
  String get learnTwoEdges => 'Cliquez deux bords';

  @override
  String get learnTwoEdgesText =>
      'Cliquez un bord, puis le bord auquel le coudre. Des flèches assorties montrent comment ils se joignent.';

  @override
  String get learnDrawTool => 'Dessiner sur le vêtement';

  @override
  String get learnDrawToolText =>
      'Choisissez **Dessiner sur le vêtement** ici, ou appuyez sur **D**.';

  @override
  String get learnDrawAcross => 'Faites glisser sur le vêtement';

  @override
  String get learnDrawAcrossText =>
      'La ligne arrive sur le patron, pour un empiècement ou une poche.';

  @override
  String get learnDragPiece => 'Faites glisser une pièce';

  @override
  String get learnDragPieceText =>
      'Maintenez **⌘** et faites glisser une pièce : de côté la fait tourner autour du corps, de haut en bas change sa hauteur.';

  @override
  String get learnSetPlacement => 'Ou réglez-la ici';

  @override
  String get learnSetPlacementText =>
      'Sélectionnez la pièce : **Sur le corps** offre les mêmes réglages, en valeurs à saisir.';

  @override
  String get learnPullPin => 'Tirer et épingler';

  @override
  String get learnPullPinText =>
      'Faites glisser le tissu pour le tirer. **⌥-clic** l’épingle là où il est.';

  @override
  String get learnYourPins => 'Vos épingles';

  @override
  String get learnYourPinsText =>
      'Sans rien sélectionner, les épingles sont listées sous **Épingles** ; **×** en retire une.';

  @override
  String get learnViews => 'Vues';

  @override
  String get learnViewsText =>
      '**Face**, **Profil**, **Dos** et **¾**, ou les touches **1**–**4**.';

  @override
  String get learnTurnZoom => 'Tourner et zoomer';

  @override
  String get learnTurnZoomText =>
      'Faites glisser autour du corps pour tourner ; molette, ou pincement sur le trackpad, pour zoomer.';

  @override
  String get learnBodyStop => 'Corps';

  @override
  String get learnBodyStopText =>
      'Choisissez une taille ou entrez des mesures : le corps est réajusté et le vêtement tombe dessus.';

  @override
  String errNotFound(String id) {
    return '$id n’existe pas.';
  }

  @override
  String errLinkedMirror(String panel) {
    return '$panel est une copie symétrique liée : modifiez sa source, ou déliez-la d’abord.';
  }

  @override
  String get errSewn =>
      'Ce bord est cousu ou fermé par une fermeture éclair : décousez d’abord la couture ou retirez la fermeture.';

  @override
  String get errNothingToUndo => 'Rien à annuler.';

  @override
  String get errNothingToRedo => 'Rien à rétablir.';

  @override
  String get errNothingToSimulate =>
      'Rien à simuler : placez au moins une pièce sur le corps.';

  @override
  String get errInverted => 'Cela retournerait une pièce sur elle-même.';

  @override
  String errRange(String min, String max) {
    return 'La valeur doit être entre $min et $max.';
  }

  @override
  String get errNotPositive => 'La valeur doit être supérieure à zéro.';

  @override
  String get errNegative => 'La valeur ne doit pas être négative.';

  @override
  String get errEmpty => 'Le nom ne doit pas être vide.';

  @override
  String get errNotSimple => 'Le contour ne doit pas se croiser.';

  @override
  String get errNotAcross =>
      'La coupe doit traverser une pièce de part en part : commencez et finissez à l’extérieur.';

  @override
  String get errMirrored =>
      'Cette pièce a une copie miroir liée : déliez d’abord la copie.';

  @override
  String get errLastGarment => 'Un projet garde au moins un vêtement.';

  @override
  String get errFileIo =>
      'Impossible de lire ou d’écrire les fichiers du projet.';

  @override
  String errImportMalformed(String format) {
    return 'Ce fichier ne peut pas être lu comme un $format.';
  }

  @override
  String get errImportBinaryDxf =>
      'Les fichiers DXF binaires ne peuvent pas être lus : enregistrez le fichier en DXF ASCII.';

  @override
  String get errImportNoPieces =>
      'Aucune pièce de patron : le dessin n’a pas de contour fermé.';

  @override
  String get errImportPagesDiffer =>
      'Les pages de ce PDF ne reprennent pas un même dessin : elles ne peuvent pas être réassemblées.';

  @override
  String get errImportEncrypted => 'Ce PDF est protégé et ne peut pas être lu.';

  @override
  String get errReadFile => 'Impossible de lire ce fichier.';

  @override
  String get errNotAProject => 'Ce dossier n’est pas un projet Rustle3D.';

  @override
  String get errTooNew =>
      'Ce projet a été créé par une version plus récente de Rustle3D.';

  @override
  String get errCorrupted => 'Ce fichier de projet est endommagé.';

  @override
  String get errAlreadyExists => 'Un projet existe déjà à cet endroit.';

  @override
  String get errArchiveInvalid =>
      'Ce fichier n’est pas une archive .zip lisible.';

  @override
  String get errArchiveNoProject =>
      'Cette archive ne contient pas de projet Rustle3D.';

  @override
  String errArchiveTooLarge(String limit) {
    return 'Cette archive est trop volumineuse pour être importée (plus de $limit Mo une fois décompressée).';
  }

  @override
  String get photo => 'Photo';

  @override
  String get photoTip => 'Faire une image photoréaliste du vêtement';

  @override
  String get photoTitle => 'Faire une photo';

  @override
  String get photoIntro =>
      'Une image photoréaliste du vêtement tel que la vue 3D le montre maintenant, dans un studio photo.';

  @override
  String get photoQuality => 'Qualité';

  @override
  String get photoQuick => 'Rapide';

  @override
  String get photoQuickHint => 'Un premier aperçu en quelques secondes';

  @override
  String get photoGood => 'Bonne';

  @override
  String get photoGoodHint => 'Assez nette pour être montrée';

  @override
  String get photoBest => 'Maximale';

  @override
  String get photoBestHint => 'Pour l\'impression et les gros plans';

  @override
  String get photoLighting => 'Éclairage';

  @override
  String get lightStudio => 'Studio';

  @override
  String get lightStudioHint =>
      'Lumière principale, d\'appoint et de contre-jour sur fond gris clair';

  @override
  String get lightSoft => 'Doux';

  @override
  String get lightSoftHint => 'Grandes lumières douces sur fond blanc';

  @override
  String get lightDramatic => 'Contrasté';

  @override
  String get lightDramaticHint => 'Une lumière dure sur fond sombre';

  @override
  String get photoSize => 'Taille';

  @override
  String get photoSizeView => 'Comme la vue 3D';

  @override
  String get photoAdvanced => 'Avancé';

  @override
  String get photoSamples => 'Échantillons par pixel';

  @override
  String get photoBounces => 'Rebonds de lumière';

  @override
  String get photoDenoise => 'Retirer le bruit';

  @override
  String get photoRender => 'Lancer le rendu';

  @override
  String photoRendering(int done, int total, String seconds) {
    return 'Rendu en cours… $done / $total échantillons · $seconds s';
  }

  @override
  String photoDone(String seconds) {
    return 'Terminé en $seconds s';
  }

  @override
  String photoStopped(int done) {
    return 'Arrêté à $done échantillons';
  }

  @override
  String photoFailed(String reason) {
    return 'La photo n\'a pas pu être rendue : $reason';
  }

  @override
  String get photoStop => 'Arrêter';

  @override
  String get photoStopTip =>
      'Arrêter d\'affiner et garder l\'image telle quelle';

  @override
  String get photoSave => 'Enregistrer l\'image…';

  @override
  String get photoClose => 'Fermer';

  @override
  String get photoCloseTip => 'Revenir à la vue 3D en direct';

  @override
  String photoSaved(String file) {
    return '$file enregistré.';
  }

  @override
  String get errPhotoTooLarge =>
      'Cette image est trop grande. Choisissez une taille plus petite (3840 × 2160 au plus).';

  @override
  String get errNoPhoto => 'Il n\'y a pas de photo à enregistrer.';

  @override
  String get learnPhoto => 'Faire une photo';

  @override
  String get learnPhotoHow =>
      'Une image photoréaliste du vêtement, à montrer à un client ou à publier.';

  @override
  String get learnPhotoStop => 'Photo';

  @override
  String get learnPhotoStopText =>
      'Tournez la vue 3D sous l\'angle qui vous plaît, cliquez sur le bouton **appareil photo**, choisissez la qualité et l\'éclairage, puis **Lancer le rendu**. L\'image s\'affine dans la vue 3D ; **Enregistrer l\'image…** la garde en PNG.';

  @override
  String get sizes => 'Tailles';

  @override
  String get sizesHint =>
      'Gradez le patron sur une gamme de tailles. Il reste tracé dans la taille de base ; les contours des autres tailles s\'emboîtent autour de chaque pièce.';

  @override
  String get addSizes => 'Ajouter une gamme de tailles';

  @override
  String get sizeChart => 'Barème';

  @override
  String get chartWomen => 'Femme';

  @override
  String get chartMen => 'Homme';

  @override
  String get sizeFrom => 'De';

  @override
  String get sizeTo => 'À';

  @override
  String get sizeBase => 'Taille de base';

  @override
  String get sizesDialogHint =>
      'Les pièces ajoutées ensuite depuis un modèle sont gradées dans toutes les tailles. Chaque taille est drapée sur un corps à ses mesures.';

  @override
  String get sizeShown => 'Affichée en 3D';

  @override
  String get sizesChange => 'Modifier';

  @override
  String get sizesRemove => 'Retirer les tailles';

  @override
  String get sizeBaseTag => 'base';

  @override
  String get gradeStep => 'Gradation par taille';

  @override
  String get gradeStepHint =>
      'De combien ce point bouge d\'une taille à la suivante.';

  @override
  String get gradeCustom =>
      'Réglée taille par taille (par un modèle ou un agent). Un pas saisi ici la remplace.';

  @override
  String get errNoSizes => 'Ajoutez d\'abord une gamme de tailles.';

  @override
  String get errUnknownSize => 'Cette taille n\'est pas dans la gamme.';

  @override
  String get errTooFewSizes => 'Une gamme compte au moins deux tailles.';

  @override
  String get errDuplicateName => 'Deux tailles portent le même nom.';

  @override
  String get errBodyOutOfRange =>
      'Une taille de cette gamme donnerait à l\'avatar des mesures hors des limites acceptées.';

  @override
  String get errBaseOffset =>
      'La taille de base ne bouge pas : c\'est le patron tel qu\'il est tracé.';

  @override
  String get sizesOnBoard => 'Afficher les tailles sur le plateau';

  @override
  String stepName(String step) {
    String _temp0 = intl.Intl.selectLogic(step, {
      'pattern_add': 'Ajouter un patron',
      'project_rename': 'Renommer le projet',
      'project_set_display_unit': 'Changer d’unité',
      'avatar_set': 'Modifier le corps',
      'avatar_remove': 'Retirer le corps',
      'panel_create_rect': 'Ajouter un rectangle',
      'panel_create_polygon': 'Ajouter un polygone',
      'panel_create_path': 'Ajouter une pièce courbe',
      'panel_rename': 'Renommer la pièce',
      'panel_translate': 'Déplacer la pièce',
      'panel_rotate': 'Faire pivoter la pièce',
      'panel_scale': 'Mettre la pièce à l’échelle',
      'panel_set_grain': 'Régler le droit-fil',
      'panel_set_seam_allowance': 'Régler la valeur de couture',
      'panel_mirror': 'Symétrie de la pièce',
      'panel_unlink': 'Délier la copie',
      'panel_place': 'Placer la pièce',
      'panel_delete': 'Supprimer la pièce',
      'panel_cut': 'Couper la pièce',
      'panel_paste': 'Coller',
      'point_move': 'Déplacer le point',
      'point_set_distance': 'Fixer la distance',
      'point_remove': 'Retirer le point',
      'edge_set_curve': 'Courber le bord',
      'edge_split': 'Ajouter un point',
      'edge_move': 'Déplacer le bord',
      'notch_add': 'Ajouter un cran',
      'notch_move': 'Déplacer le cran',
      'notch_remove': 'Retirer le cran',
      'internal_line_add': 'Tracer une ligne',
      'internal_line_remove': 'Retirer la ligne',
      'seam_add': 'Coudre',
      'seam_remove': 'Découdre',
      'seam_flip': 'Inverser la couture',
      'seam_set_gathered': 'Froncer la couture',
      'edge_lengthen': 'Rallonger',
      'edge_set_length': 'Fixer la longueur',
      'seam_take_in': 'Reprendre la couture',
      'seam_match_length': 'Égaliser les longueurs',
      'pin_add': 'Épingler',
      'pin_remove': 'Retirer l’épingle',
      'garment_set_fabric': 'Changer de tissu',
      'garment_arrange': 'Disposer les pièces',
      'garment_set_color': 'Changer la couleur',
      'garment_add': 'Ajouter un vêtement',
      'garment_rename': 'Renommer le vêtement',
      'garment_remove': 'Retirer le vêtement',
      'garment_set_layer': 'Changer de couche',
      'panel_set_garment': 'Changer de vêtement',
      'grading_set_sizes': 'Définir les tailles',
      'grading_use_chart': 'Utiliser un barème',
      'grading_clear': 'Retirer les tailles',
      'grading_show_size': 'Afficher la taille',
      'point_set_grade': 'Graduer le point',
      'template_apply': 'Ajouter un modèle',
      'pattern_import': 'Importer un patron',
      'batch': 'Plusieurs modifications',
      'constraint_add': 'Ajouter une contrainte',
      'constraint_remove': 'Retirer une contrainte',
      'constraint_set_value': 'Changer une cote',
      'point_set_smooth': 'Point lisse',
      'panel_unfold': 'Déplier la pièce',
      'guide_add': 'Ajouter une ligne de construction',
      'guide_move': 'Déplacer une ligne de construction',
      'guide_remove': 'Retirer une ligne de construction',
      'panel_create_ellipse': 'Ajouter une ellipse',
      'panel_create_sector': 'Ajouter une jupe soleil',
      'edge_set_arc': 'Faire un arc',
      'constraint_set_formula': 'Formule de cote',
      'variable_set': 'Modifier une variable',
      'variable_remove': 'Retirer une variable',
      'seam_set_ease': 'Régler l’embu',
      'dart_add': 'Ajouter une pince',
      'pleat_add': 'Ajouter un pli',
      'panel_add_hole': 'Découper un trou',
      'panel_remove_hole': 'Boucher un trou',
      'panel_boolean': 'Joindre ou découper',
      'other': '$step',
    });
    return '$_temp0';
  }

  @override
  String undoStep(String name) {
    return 'Annuler $name  ⌘Z';
  }

  @override
  String redoStep(String name) {
    return 'Rétablir $name  ⇧⌘Z';
  }

  @override
  String get nothingToUndo => 'Rien à annuler';

  @override
  String get nothingToRedo => 'Rien à rétablir';

  @override
  String get historyTip => 'Historique : revenir à n’importe quelle étape';

  @override
  String get historyTitle => 'Historique';

  @override
  String get historyStart => 'Ouverture';

  @override
  String get historyEmpty => 'Aucune modification pour l’instant.';

  @override
  String copied(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count pièces copiées',
      one: '1 pièce copiée',
    );
    return '$_temp0, avec les coutures entre elles. ⌘V les colle, ici ou dans un autre projet.';
  }

  @override
  String pasted(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count pièces collées',
      one: '1 pièce collée',
    );
    return '$_temp0.';
  }

  @override
  String get nothingToPaste =>
      'Rien à coller : copiez d’abord des pièces (⌘C).';

  @override
  String get selectToCopy => 'Sélectionnez les pièces à copier.';

  @override
  String get errCancelled => 'Arrêté : rien n’a été modifié.';

  @override
  String get busyStop => 'Arrêter';

  @override
  String get busyStep => 'En cours…';

  @override
  String busySizes(String size, int done, int total) {
    return 'Tracé de la taille $size ($done sur $total)';
  }

  @override
  String busySizesStart(int total) {
    return 'Tracé de chaque taille ($total)';
  }

  @override
  String get busyBodyFit => 'Ajustement du corps aux mesures…';

  @override
  String get busyDrape => 'Mise en place des pièces autour du corps…';

  @override
  String get busyExport => 'Export en cours…';

  @override
  String get busyImport => 'Lecture du patron…';

  @override
  String get busyOpening => 'Ouverture du projet';

  @override
  String get busyTemplate => 'Ajout du modèle';

  @override
  String get busySizesTitle => 'Réglage des tailles';

  @override
  String get busyBodyTitle => 'Modification du corps';

  @override
  String get busyDrapeTitle => 'Lancement du drapé';

  @override
  String get bodyFitting => 'Ajustement du corps…';

  @override
  String get toolShapes => 'Formes';

  @override
  String get toolEllipse => 'Ellipse';

  @override
  String get toolEllipseHint =>
      'Un cercle ou une ellipse aux dimensions saisies';

  @override
  String get toolSector => 'Jupe soleil';

  @override
  String get toolSectorHint =>
      'Des pièces en anneau d’après le tour de taille et la longueur de jupe';

  @override
  String get toolArc => 'Arc';

  @override
  String get toolArcHint => 'Cliquez une arête pour en faire un arc de cercle';

  @override
  String get toolGuide => 'Ligne de construction';

  @override
  String get toolGuideHint =>
      'Cliquez deux points : une ligne pour tracer, jamais coupée ni cousue';

  @override
  String get hintGuide =>
      'Ligne de construction : cliquez deux points · Échap annule · V pour arrêter';

  @override
  String get toolDimension => 'Cote';

  @override
  String get toolDimensionHint =>
      'Cliquez un bord, deux coins ou deux bords, puis là où va la cote ; tapez sa valeur ou une formule';

  @override
  String get hintDimension =>
      'Cote : cliquez un bord (sa longueur), un coin puis un autre (la distance) ou deux bords (l’angle) · V pour arrêter';

  @override
  String get hintDimensionPlace =>
      'Cote : cliquez là où elle va (entre deux coins, tirez-la au-dessus ou à côté pour une largeur ou une hauteur), puis tapez sa valeur · Échap annule';

  @override
  String get hintArc => 'Arc : cliquez une arête · V pour arrêter';

  @override
  String get toastGuideAdded => 'Ligne de construction ajoutée.';

  @override
  String get symmetricHalf => 'Demi-pièce';

  @override
  String get symmetricHalfTip =>
      'Tracez la moitié d’une pièce symétrique : l’arête de fermeture est l’axe, l’autre moitié suit';

  @override
  String get width => 'Largeur';

  @override
  String get height => 'Hauteur';

  @override
  String get sectorWaist => 'Tour de taille';

  @override
  String get sectorLength => 'Longueur de jupe';

  @override
  String get sectorFull => 'Soleil';

  @override
  String get sectorHalf => 'Demi-soleil';

  @override
  String get sectorQuarter => 'Quart de soleil';

  @override
  String get sectorHint =>
      'Une jupe soleil se coupe en deux demi-pièces, à coudre sur les côtés.';

  @override
  String get arcTitle => 'Arc';

  @override
  String get arcBulge => 'Flèche';

  @override
  String get arcHint =>
      'Distance entre le milieu de l’arc et la ligne droite ; négative, l’arc rentre vers l’intérieur.';

  @override
  String get constraints => 'Contraintes';

  @override
  String get constraintsEmpty =>
      'Choisissez un coin ou une arête sur le patron (⌘-clic pour un second) pour les contraindre.';

  @override
  String constraintsFreedom(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count degrés de liberté restants',
      one: '1 degré de liberté restant',
      zero: 'Entièrement contraint',
    );
    return '$_temp0';
  }

  @override
  String get constraintsRedundant =>
      'Certaines contraintes en répètent d’autres.';

  @override
  String get constraintBroken => 'Non respectée';

  @override
  String constraintKind(String kind) {
    String _temp0 = intl.Intl.selectLogic(kind, {
      'fixed': 'Fixe',
      'coincident': 'Joints',
      'horizontal': 'Horizontale',
      'vertical': 'Verticale',
      'parallel': 'Parallèles',
      'perpendicular': 'D’équerre',
      'tangent': 'Lisse',
      'equal_length': 'Même longueur',
      'on_edge': 'Sur l’arête',
      'distance': 'Distance',
      'width': 'Largeur',
      'height': 'Hauteur',
      'length': 'Longueur',
      'angle': 'Angle',
      'on_guide': 'Sur la ligne',
      'notch_distance': 'Distance du cran',
      'guide_distance': 'Distance à la ligne',
      'other': '$kind',
    });
    return '$_temp0';
  }

  @override
  String get barHint =>
      'Choisissez des coins, des arêtes, des crans ou des lignes de construction sur le patron (⇧- ou ⌘-clic en ajoute) pour les contraindre.';

  @override
  String barCorners(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count coins',
      one: '1 coin',
    );
    return '$_temp0';
  }

  @override
  String barEdges(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count arêtes',
      one: '1 arête',
    );
    return '$_temp0';
  }

  @override
  String barNotches(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count crans',
      one: '1 cran',
    );
    return '$_temp0';
  }

  @override
  String barLines(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count lignes',
      one: '1 ligne',
    );
    return '$_temp0';
  }

  @override
  String get barFixedHint =>
      'Le coin reste où il est quand le dessin est recalculé.';

  @override
  String get barSmoothHint =>
      'Sans angle : le contour passe en douceur par le point.';

  @override
  String get barCornerHint =>
      'De nouveau un angle : les deux arêtes ne sont plus alignées.';

  @override
  String get barRemovePointHint =>
      'Supprimer le coin ; ses deux arêtes n’en font plus qu’une.';

  @override
  String get barWidthHint =>
      'Coter l’écart en largeur entre les deux coins, puis le taper.';

  @override
  String get barHeightHint =>
      'Coter l’écart en hauteur entre les deux coins, puis le taper.';

  @override
  String get barDistanceHint =>
      'Coter la distance en ligne droite entre les deux coins, puis la taper.';

  @override
  String get barJoinedHint =>
      'Les deux coins au même endroit (sur deux pièces, en général).';

  @override
  String get barLengthHint =>
      'Coter la longueur de l’arête le long de sa courbe, puis la taper.';

  @override
  String get barHorizontalHint =>
      'Les extrémités de l’arête à la même hauteur.';

  @override
  String get barVerticalHint =>
      'Les extrémités de l’arête l’une au-dessus de l’autre.';

  @override
  String get barAngleHint =>
      'Coter l’angle entre les deux arêtes, puis le taper.';

  @override
  String get barSameLengthHint =>
      'Les arêtes aussi longues que la première choisie.';

  @override
  String get barParallelHint => 'Les arêtes parallèles à la première choisie.';

  @override
  String get barSquareHint => 'Les deux arêtes à angle droit.';

  @override
  String get barOnEdgeHint =>
      'Le coin sur l’arête (sur son prolongement, pour une arête droite).';

  @override
  String get barNotchStart => 'Depuis le début';

  @override
  String get barNotchStartHint =>
      'Coter la distance du cran le long de son arête depuis le début de l’arête ; il la garde quand l’arête change.';

  @override
  String get barNotchEnd => 'Depuis la fin';

  @override
  String get barNotchEndHint =>
      'Coter la distance du cran le long de son arête depuis la fin de l’arête ; il la garde quand l’arête change.';

  @override
  String get barRemoveNotch => 'Supprimer le cran';

  @override
  String get barRemoveNotchHint => 'Supprimer le cran.';

  @override
  String get barGuideDistanceHint =>
      'Coter la distance du coin à la ligne, puis la taper ; déplacer la ligne déplace le coin.';

  @override
  String get barOnGuideHint =>
      'Le coin sur la ligne ; déplacer la ligne déplace le coin.';

  @override
  String get barLevel => 'À l’horizontale';

  @override
  String get barLevelHint =>
      'Tourner la ligne autour de sa première extrémité pour la mettre à l’horizontale.';

  @override
  String get barUpright => 'À la verticale';

  @override
  String get barUprightHint =>
      'Tourner la ligne autour de sa première extrémité pour la mettre à la verticale.';

  @override
  String get barRemoveLine => 'Supprimer la ligne';

  @override
  String get barRemoveLineHint =>
      'Supprimer la ligne de construction (et ses contraintes).';

  @override
  String get barRotateHint =>
      'Tourner la pièce d’un angle saisi. La poignée ronde de son cadre la tourne à la main.';

  @override
  String get barScaleHint =>
      'Élargir ou allonger la pièce d’un pourcentage. Les coins de son cadre la redimensionnent à la main.';

  @override
  String get barMirrorHint =>
      'Ajouter une copie symétrique qui suit cette pièce.';

  @override
  String get barJoinHint =>
      'Fusionner une autre pièce avec celle-ci : choisissez-la dans la liste.';

  @override
  String get barCutOutHint =>
      'Retirer la forme d’une autre pièce de celle-ci : choisissez-la dans la liste.';

  @override
  String get barDeleteHint => 'Supprimer la pièce.';

  @override
  String get barNothing => 'Aucune sélection';

  @override
  String get barShortRotate => 'Pivoter';

  @override
  String get barShortScale => 'Échelle';

  @override
  String get barShortFromStart => 'Du début';

  @override
  String get barShortFromEnd => 'De la fin';

  @override
  String get barDeleteAllHint => 'Supprimer les pièces sélectionnées.';

  @override
  String get dimensionRename =>
      'Nom : les formules peuvent utiliser cette cote';

  @override
  String get dimensionNameTitle => 'Nom de la cote';

  @override
  String get dimensionNameHelp =>
      'Lettres, chiffres et _, en commençant par une lettre. Les formules l’utilisent alors comme une variable : la longueur d’une tête de manche peut être « armhole + 1.5 cm ». Vide : pas de nom.';

  @override
  String get constrain => 'Contraindre';

  @override
  String get removeConstraint => 'Retirer';

  @override
  String get pointSmooth => 'Lisse';

  @override
  String get pointCorner => 'Coin';

  @override
  String get pointRemove => 'Retirer le point';

  @override
  String get symmetricPiece => 'Pièce symétrique';

  @override
  String get unlinkHalves => 'Séparer les moitiés';

  @override
  String get unfold => 'Déplier ici';

  @override
  String get makeArc => 'Arc';

  @override
  String snapKind(String kind) {
    String _temp0 = intl.Intl.selectLogic(kind, {
      'point': 'Point',
      'midpoint': 'Milieu',
      'crossing': 'Croisement',
      'edge': 'Sur l’arête',
      'guide': 'Sur la ligne',
      'axis': 'Sur l’axe',
      'horizontal': 'Même hauteur',
      'vertical': 'Aligné',
      'other': '$kind',
    });
    return '$_temp0';
  }

  @override
  String get errDerived =>
      'Cette partie suit la moitié tracée ou la pièce d’origine : modifiez celle-ci.';

  @override
  String get errUnsolvable =>
      'Ces contraintes ne peuvent pas toutes tenir : rien n’a été changé.';

  @override
  String get errSymmetric => 'Pièce symétrique : séparez d’abord ses moitiés.';

  @override
  String get errNotStraight => 'Il faut une arête droite.';

  @override
  String get errDuplicate => 'Déjà contraint de cette façon.';

  @override
  String get formulaTip =>
      'Formule : calculer cette valeur d’après les mesures du corps';

  @override
  String get formulaTitle => 'Formule';

  @override
  String get formulaHelp =>
      'Mesures : height (stature), bust (poitrine), waist (taille), hips (hanches). Unités : mm, cm, in. + − × ÷, min, max. Vos variables aussi. Vide : la valeur reste telle quelle.';

  @override
  String get variables => 'Variables';

  @override
  String get variablesHint =>
      'Des noms pour les formules, comme ease = 2 cm. Les pièces à formules se redessinent pour un nouveau corps et dans chaque taille.';

  @override
  String get variable => 'Variable';

  @override
  String get variableName => 'Nom';

  @override
  String get variableAdd => 'Ajouter une variable';

  @override
  String get errGradedByFormula =>
      'Ce point est gradué par des formules : modifiez-les plutôt.';

  @override
  String errFormulaSyntax(String at) {
    return 'La formule est illisible au caractère $at.';
  }

  @override
  String errFormulaUnknown(String name) {
    return '« $name » n’est ni une mesure ni une variable.';
  }

  @override
  String errFormulaNoBody(String name) {
    return '« $name » demande les mesures du corps : saisissez-les d’abord.';
  }

  @override
  String errFormulaCycle(String name) {
    return '« $name » se calcule à partir de lui-même.';
  }

  @override
  String get errFormulaUnit =>
      'Les unités ne correspondent pas (une longueur prend une unité, comme 10 mm).';

  @override
  String get errFormulaDomain =>
      'La formule n’a pas de valeur (une division par zéro ?).';

  @override
  String errFormulaRange(String value) {
    return 'La formule donne $value : trop petit pour cette cote.';
  }

  @override
  String errFormulaBadName(String name) {
    return '« $name » ne peut pas nommer une variable : une lettre, puis des lettres, chiffres ou _.';
  }

  @override
  String get toolDetails => 'Pinces et plis';

  @override
  String get toolDart => 'Pince';

  @override
  String get toolDartHint =>
      'Cliquez une arête à l’endroit de la pince : tracée, pliée et cousue';

  @override
  String get toolPleat => 'Pli';

  @override
  String get toolPleatHint =>
      'Cliquez une arête là où commence le pli : plié et cousu';

  @override
  String get toolHole => 'Trou';

  @override
  String get toolHoleHint =>
      'Sélectionnez une pièce, puis cliquez les coins d’un trou à l’intérieur';

  @override
  String get hintDart =>
      'Pince : cliquez une arête à son emplacement · V pour arrêter';

  @override
  String get hintPleat =>
      'Pli : cliquez une arête là où il commence · V pour arrêter';

  @override
  String get hintHole =>
      'Trou : cliquez ses coins dans la pièce sélectionnée, Entrée pour le découper · Échap annule';

  @override
  String get toastHoleAdded => 'Trou découpé.';

  @override
  String get holeSelectFirst => 'Sélectionnez d’abord la pièce à trouer.';

  @override
  String get dartWidth => 'Largeur au bord';

  @override
  String get dartLength => 'Longueur';

  @override
  String get pleatDepth => 'Profondeur';

  @override
  String get pleatStitched => 'Piqué sur';

  @override
  String get pleatKnife => 'Plat';

  @override
  String get pleatBox => 'Creux';

  @override
  String get join => 'Joindre';

  @override
  String get cutOut => 'Découper';

  @override
  String get ease => 'Embu';

  @override
  String get errOverlaps => 'Ce tronçon est déjà cousu.';

  @override
  String get errNotWholeEdges =>
      'Cela marche sur les coutures entre deux arêtes entières.';

  @override
  String get errApart => 'Le résultat ferait plusieurs pièces, ou rien.';

  @override
  String get errHasHoles => 'Cette pièce a des trous : bouchez-les d’abord.';

  @override
  String get rotatePiece => 'Faire pivoter';

  @override
  String get rotatePieceHint =>
      'Dans le sens inverse des aiguilles d’une montre ; un angle négatif tourne dans l’autre sens.';

  @override
  String get rotateAngle => 'Angle';

  @override
  String get scalePiece => 'Mettre à l’échelle';

  @override
  String get scaleAcross => 'En largeur';

  @override
  String get scaleUpDown => 'En hauteur';

  @override
  String get errConstrained =>
      'Cette pièce a des cotes ou des contraintes qui ne tiendraient plus : retirez-les d’abord.';

  @override
  String get errNotJoined => 'Ces arêtes ne se suivent pas sur une même pièce.';

  @override
  String get close => 'Fermer';

  @override
  String get marker => 'Placement';

  @override
  String get exportMarker => 'Placement : les pièces posées sur le tissu…';

  @override
  String get markerHint =>
      'Les pièces, avec leurs valeurs de couture, posées sur le tissu dans le droit fil. Choisissez les tailles à couper et la laize du tissu : vous obtenez la longueur à acheter et son coût.';

  @override
  String get markerGarment => 'Vêtement';

  @override
  String get markerSizes => 'Tailles à couper';

  @override
  String get markerWidth => 'Laize du tissu';

  @override
  String get markerLayOut => 'Placer';

  @override
  String get markerLength => 'Longueur';

  @override
  String get markerEfficiency => 'Tissu utilisé';

  @override
  String get markerYardage => 'À acheter';

  @override
  String get markerCost => 'Coût';

  @override
  String get markerNoPrice =>
      'Donnez au tissu un prix au mètre (inspecteur, sans sélection, sous le vêtement) pour voir le coût.';

  @override
  String get markerSaveSvg => 'Enregistrer en SVG';

  @override
  String get markerSavePdf => 'Enregistrer en PDF';

  @override
  String get tpDecimal => ',';

  @override
  String get tpSizes => 'Tailles';

  @override
  String get tpSample => 'base';

  @override
  String get tpViews => 'Vues';

  @override
  String get tpFront => 'Devant';

  @override
  String get tpBack => 'Dos';

  @override
  String get tpColourways => 'Coloris';

  @override
  String get tpMaterials => 'Nomenclature';

  @override
  String get tpFabrics => 'Tissus';

  @override
  String get tpGarment => 'Vêtement';

  @override
  String get tpFabric => 'Tissu';

  @override
  String get tpColour => 'Couleur';

  @override
  String get tpWeight => 'Grammage (g/m²)';

  @override
  String get tpWidth => 'Laize (cm)';

  @override
  String get tpPrice => 'Prix au mètre';

  @override
  String get tpConsumption => 'Tissu par vêtement (m)';

  @override
  String get tpCost => 'Coût par vêtement';

  @override
  String get tpConsumptionBySize => 'Tissu par vêtement et par taille (m)';

  @override
  String get tpTrims => 'Fournitures';

  @override
  String get tpNoTrims => 'Aucune fourniture.';

  @override
  String get tpMeasurements => 'Mesures';

  @override
  String get tpSizeChart => 'Tableau des tailles (corps, cm)';

  @override
  String get tpSize => 'Taille';

  @override
  String get tpHeight => 'Stature';

  @override
  String get tpBust => 'Tour de poitrine';

  @override
  String get tpWaist => 'Tour de taille';

  @override
  String get tpHips => 'Tour de bassin';

  @override
  String get tpPieces => 'Pièces';

  @override
  String get tpPiece => 'Pièce';

  @override
  String get tpLength => 'Longueur (cm)';

  @override
  String get tpPieceWidth => 'Largeur (cm)';

  @override
  String get tpPerimeter => 'Ligne de couture (cm)';

  @override
  String get tpArea => 'Surface (cm²)';

  @override
  String get tpCutArea => 'Surface coupée (cm²)';

  @override
  String get tpAllowance => 'Valeur de couture (mm)';

  @override
  String get tpMass => 'Masse (g)';

  @override
  String get tpPiecesBySize => 'Pièces par taille : longueur x largeur (cm)';

  @override
  String get tpPattern => 'Patron';

  @override
  String get tpSewing => 'Notes de montage';

  @override
  String get tpSeams => 'Coutures';

  @override
  String get tpSideA => 'Côté A';

  @override
  String get tpSideB => 'Côté B';

  @override
  String get tpLengthA => 'Longueur A (mm)';

  @override
  String get tpLengthB => 'Longueur B (mm)';

  @override
  String get tpEase => 'Embu (mm)';

  @override
  String get tpGathered => 'Froncé';

  @override
  String get tpYes => 'oui';

  @override
  String get tpNoSeams => 'Pas encore de coutures.';

  @override
  String get tpNotes => 'Notes';

  @override
  String get exportTechpack => 'Dossier technique (PDF)…';

  @override
  String get exportProjector => 'Projecteur : le patron à l’échelle 1…';

  @override
  String get techpackLetter => 'Pages Letter';

  @override
  String get garmentNotes => 'Notes';

  @override
  String get garmentNotesHint =>
      'Notes de montage et de finition, imprimées dans le dossier technique';

  @override
  String get projectorHelp =>
      'Faites glisser ou utilisez les flèches pour amener le patron sous le projecteur (Maj : 10 cm). Origine : retour au centre. M : miroir · R : tourner · I : contraste · C : calibrer · Échap : fermer.';

  @override
  String get projectorCalibrate => 'Calibrer';

  @override
  String get projectorMirror => 'Miroir';

  @override
  String get projectorTurn => 'Tourner';

  @override
  String get projectorContrast => 'Contraste';

  @override
  String get projectorCalibrateHint =>
      'Le carré ci-dessous devrait mesurer 20 cm de côté sur la table. Mesurez ses côtés avec une règle et saisissez-les.';

  @override
  String get projectorMeasuredWidth => 'Largeur mesurée';

  @override
  String get projectorMeasuredHeight => 'Hauteur mesurée';

  @override
  String get projectorApply => 'Appliquer';

  @override
  String get projectorReset => 'Réinitialiser';

  @override
  String get projectorNothing => 'Aucune pièce à projeter.';

  @override
  String get fabricBought => 'À l’achat';

  @override
  String get fabricWeight => 'Grammage';

  @override
  String get fabricWidth => 'Laize';

  @override
  String get fabricPrice => 'Prix / m';

  @override
  String get cutArea => 'Surface coupée';

  @override
  String get pieceMass => 'Masse';

  @override
  String get errMarkerTooNarrow =>
      'Une pièce est plus large que le tissu : choisissez une laize plus grande.';

  @override
  String garmentTotals(int pieces, String area, String mass) {
    return '$pieces pièces · $area m² de tissu coupé · $mass g';
  }

  @override
  String get fitTip => 'Voir où le vêtement serre';

  @override
  String get fitOff => 'Couleurs du tissu';

  @override
  String get fitStrain => 'Allongement';

  @override
  String get fitGap => 'Écart au corps';

  @override
  String get fitStrainTitle => 'Allongement par rapport au patron';

  @override
  String get fitGapTitle => 'Écart au corps';

  @override
  String get fitLoose => 'ample';

  @override
  String get fitTight => 'serré';

  @override
  String fitTightest(String value, String piece) {
    return 'Le plus serré : $value sur $piece';
  }

  @override
  String get toolTape => 'Mètre ruban';

  @override
  String get toolTapeHint =>
      'Cliquez sur le vêtement 3D : son tour à cet endroit, celui du corps et l\'aisance ; ou une longueur entre deux points';

  @override
  String get hintTape =>
      'Mètre ruban : cliquez sur le vêtement 3D pour en mesurer le tour, ou deux points pour une longueur · faites glisser pour tourner la vue · V pour arrêter';

  @override
  String get tapeHint => 'Cliquez sur le vêtement drapé.';

  @override
  String get tapeClose => 'Ranger le mètre';

  @override
  String get tapeHeight => 'Hauteur';

  @override
  String get tapeGarment => 'Vêtement';

  @override
  String get tapeBody => 'Corps';

  @override
  String get tapeEase => 'Aisance';

  @override
  String get tapeGap => 'Écart ici';

  @override
  String get tapeOpen =>
      'Le vêtement ne fait pas le tour ici : pas d\'aisance.';

  @override
  String get tapeRound => 'Tour';

  @override
  String get tapeLength => 'Longueur';

  @override
  String get tapeFirstPoint =>
      'Cliquez le premier point sur le vêtement drapé.';

  @override
  String get tapeSecondPoint => 'Cliquez le second point.';

  @override
  String get tapeAlong => 'Le long du tissu';

  @override
  String get tapeTaut => 'Mètre tendu';

  @override
  String get tapeStraight => 'En ligne droite';

  @override
  String get tapeNotJoined =>
      'Le tissu ne relie pas ces points : ligne droite seulement.';

  @override
  String get printTitle => 'Imprimé';

  @override
  String get printHint =>
      'Suit le droit-fil de chaque pièce, à sa taille réelle.';

  @override
  String get printNone => 'Sans imprimé';

  @override
  String get printStripes => 'Rayures';

  @override
  String get printGingham => 'Vichy';

  @override
  String get printDots => 'Pois';

  @override
  String get printTwill => 'Sergé';

  @override
  String get printImage => 'Votre image (PNG, JPEG)…';

  @override
  String get printImages => 'Images';

  @override
  String get printMissing =>
      'L\'image est illisible : la couleur unie est affichée.';

  @override
  String get printWidth => 'Largeur du motif';

  @override
  String get printAngle => 'Angle au droit-fil';

  @override
  String get piecePrint => 'Imprimé sur cette pièce';

  @override
  String get piecePrintAcross => 'Décalage en largeur';

  @override
  String get piecePrintUp => 'Décalage en hauteur';

  @override
  String get piecePrintTurn => 'Rotation';

  @override
  String get errTextureFormat => 'Ce fichier n\'est pas une image PNG ou JPEG.';

  @override
  String get errTextureBad => 'Cette image est illisible.';

  @override
  String get errTextureTooLarge =>
      'Cette image est trop grande (16384 pixels de côté au plus).';

  @override
  String get toolTrims => 'Fournitures';

  @override
  String get toolButton => 'Bouton';

  @override
  String get toolButtonHint =>
      'Cliquez où va le bouton, puis où il se boutonne : le bout de la boutonnière, sur la pièce qui croise par-dessus';

  @override
  String get toolZipper => 'Fermeture éclair';

  @override
  String get toolZipperHint =>
      'Cliquez un bord près du bas de la fermeture, puis le bord qu\'elle ferme avec lui';

  @override
  String get toastButtonAdded => 'Bouton ajouté.';

  @override
  String get toastZipperAdded => 'Fermeture éclair ajoutée.';

  @override
  String get buttonClickPiece => 'Cliquez sur une pièce.';

  @override
  String get trimsTitle => 'Boutons et fermetures';

  @override
  String trimButton(String mm) {
    return 'Bouton · $mm mm';
  }

  @override
  String trimButtonhole(String mm, String piece) {
    return 'Boutonnière · bouton de $mm mm sur $piece';
  }

  @override
  String get trimButtoned => 'Boutonné';

  @override
  String trimZipper(String cm) {
    return 'Fermeture éclair · $cm cm';
  }

  @override
  String get trimOpen => 'Ouverte';

  @override
  String get buttonAll => 'Tout boutonner';

  @override
  String get unbuttonAll => 'Tout déboutonner';

  @override
  String get trimRemove => 'Retirer';

  @override
  String get tpTrim => 'Fourniture';

  @override
  String get tpTrimSize => 'Taille';

  @override
  String get tpQuantity => 'Quantité';

  @override
  String get tpPlacement => 'Sur';

  @override
  String get tpButton => 'Bouton';

  @override
  String get tpZipper => 'Fermeture éclair';

  @override
  String get errOutsidePiece => 'Placez-le à l\'intérieur de la pièce.';

  @override
  String get bodyMenuMeasurements => 'Mesures…';

  @override
  String get bodyMenuImport => 'Importer un corps…';

  @override
  String get bodyMenuCheck => 'Vérifier le corps…';

  @override
  String get bodyMenuPoints => 'Afficher les points de placement';

  @override
  String get bodyMenuBuiltin => 'Revenir au corps intégré';

  @override
  String get bodyFiles => 'Corps 3D (OBJ, glTF, GLB)';

  @override
  String get bodyImported => 'Corps importé : vérifiez-le avant de draper.';

  @override
  String get bodyBuiltinBack => 'Retour au corps intégré.';

  @override
  String get bodyCheckTitle => 'Vérifier le corps';

  @override
  String bodyCheckSummary(String file, String triangles, String height) {
    return '$file · $triangles triangles · $height m de haut';
  }

  @override
  String get bodyCheckReady => 'Il peut être drapé.';

  @override
  String get bodyCheckNotReady =>
      'Il ne peut pas encore être drapé : voir ci-dessous.';

  @override
  String get bodyCheckAllGood => 'Rien à corriger.';

  @override
  String get bodyReading => 'Lecture du fichier';

  @override
  String get bodyUnits => 'Unités';

  @override
  String get unitMetres => 'Mètres';

  @override
  String get unitCentimetres => 'Centimètres';

  @override
  String get unitMillimetres => 'Millimètres';

  @override
  String get unitInches => 'Pouces';

  @override
  String unitOther(String metres) {
    return '$metres m par unité';
  }

  @override
  String get bodyUp => 'Axe vertical';

  @override
  String get bodyTurn => 'Rotation';

  @override
  String get bodyPoints => 'Points de placement';

  @override
  String get bodyPointsHint =>
      'Dans le corps, en cm : largeur (gauche +), hauteur, profondeur (devant +). Déplacez-les dans la vue 3D avec Corps › Afficher les points de placement.';

  @override
  String get bodyGuessAgain => 'Replacer';

  @override
  String get bodyHasWarnings => 'Le corps a des points à vérifier.';

  @override
  String get pointChest => 'Poitrine';

  @override
  String get pointWaist => 'Taille';

  @override
  String get pointHips => 'Hanches';

  @override
  String get pointCrotch => 'Entrejambe';

  @override
  String get pointLeftShoulder => 'Épaule gauche';

  @override
  String get pointLeftElbow => 'Coude gauche';

  @override
  String get pointLeftWrist => 'Poignet gauche';

  @override
  String get pointRightShoulder => 'Épaule droite';

  @override
  String get pointRightElbow => 'Coude droit';

  @override
  String get pointRightWrist => 'Poignet droit';

  @override
  String get pointLeftThigh => 'Cuisse gauche';

  @override
  String get pointLeftShin => 'Tibia gauche';

  @override
  String get pointRightThigh => 'Cuisse droite';

  @override
  String get pointRightShin => 'Tibia droit';

  @override
  String checkFileUnavailable(String file) {
    return 'Le fichier du corps $file manque ou a changé : importez-le à nouveau.';
  }

  @override
  String checkMeshOpen(String holes, String edges) {
    return '$holes trous dans la surface ($edges arêtes ouvertes) : refermés pour le tissu au mieux.';
  }

  @override
  String checkMeshNonManifold(String edges) {
    return '$edges arêtes sont partagées par plus de deux triangles.';
  }

  @override
  String checkMeshPieces(String pieces) {
    return '$pieces parties détachées en plus du corps.';
  }

  @override
  String checkMeshHeavy(String triangles) {
    return '$triangles triangles : les vues et le drapé seront plus lents.';
  }

  @override
  String checkMeshCoarse(String triangles) {
    return 'Seulement $triangles triangles : le corps paraîtra anguleux.';
  }

  @override
  String checkMeshReoriented(String triangles) {
    return '$triangles triangles ont été retournés vers l\'extérieur comme leurs voisins.';
  }

  @override
  String get checkMeshInsideOut =>
      'La surface était tournée vers l\'intérieur : elle a été retournée.';

  @override
  String checkMeshDegenerate(String triangles) {
    return '$triangles triangles plats sont ignorés.';
  }

  @override
  String checkScaleHeight(String height) {
    return '$height m de haut : les unités sont-elles justes ?';
  }

  @override
  String get checkOrientationUp =>
      'Le corps est plus large que haut : l\'axe vertical est-il juste ?';

  @override
  String get checkOrientationFacing =>
      'Les pieds pointent vers l\'arrière : tournez le corps de 180°.';

  @override
  String checkPointsOutside(String point) {
    return '$point est hors du corps.';
  }

  @override
  String checkPointsSide(String point) {
    return '$point est du mauvais côté du corps.';
  }

  @override
  String checkMeasurementsClamped(String measurement) {
    return 'Une mesure ($measurement) était hors limites et y a été ramenée.';
  }

  @override
  String get checkMotionFileSkeleton => 'Bouge avec le squelette du fichier.';

  @override
  String get checkMotionPoints =>
      'Bouge avec un squelette placé par les points de placement : déplacez-les si un membre plie au mauvais endroit.';

  @override
  String errModelMalformed(String format, String detail) {
    return 'Ce fichier $format est illisible : $detail';
  }

  @override
  String get errModelExternalBuffers =>
      'Ce glTF range ses données dans des fichiers séparés : enregistrez-le en un seul .glb.';

  @override
  String errModelCompressed(String extension) {
    return 'Ce glTF est compressé ($extension) : enregistrez-le sans compression.';
  }

  @override
  String get errModelEmpty => 'Ce fichier ne contient aucun triangle.';

  @override
  String errModelTooLarge(String triangles, String limit) {
    return 'Ce corps a $triangles triangles, plus de $limit : simplifiez-le d\'abord.';
  }

  @override
  String errBodyUnavailable(String file) {
    return 'Le fichier du corps $file manque ou a changé : importez-le à nouveau.';
  }

  @override
  String get errNoImportedBody => 'L\'avatar n\'a pas de corps importé.';

  @override
  String get exportGame => 'Vêtement pour moteur de jeu (glTF)…';

  @override
  String get gameTitle => 'Exporter pour un moteur de jeu';

  @override
  String get gameIntro =>
      'Le vêtement tel que la vue 3D le montre, dans un fichier GLB : des niveaux de détail plus légers, ses plis enregistrés dans des normal maps, et il bouge avec le squelette du corps.';

  @override
  String get gameEngine => 'Moteur';

  @override
  String get gameGeneric => 'Générique';

  @override
  String get gameLods => 'Niveaux de détail';

  @override
  String get gameLodsHint =>
      'Pourcentages des triangles, le premier à 100 : par exemple 100, 50, 20. Les bords des pièces ne bougent jamais.';

  @override
  String get gameNormalMap => 'Normal map';

  @override
  String get gameNoMap => 'Aucune';

  @override
  String get gameNormalMapHint =>
      'Les plis du vêtement complet, enregistrés pour chaque niveau plus léger.';

  @override
  String get gameSides => 'Faces arrière';

  @override
  String get gameDoubleSided => 'Double face';

  @override
  String get gameThickness => 'Épaisseur';

  @override
  String get gameThicknessMm => 'Épaisseur';

  @override
  String get gameSkin => 'Lier au squelette du corps';

  @override
  String get gameSkinHint =>
      'Le vêtement bouge avec le corps, avec les mouvements intégrés et ceux capturés du projet en animations.';

  @override
  String get gameAvatar => 'Inclure le corps';

  @override
  String get gameExport => 'Exporter…';

  @override
  String get mm => 'mm';

  @override
  String get errLods =>
      'Niveaux de détail : commencer à 100 puis décroître, chacun au-dessus de 0, cinq au plus.';

  @override
  String get errTextureSize =>
      'Choisissez aucune map, ou une puissance de deux de 256 à 4096.';

  @override
  String get errNoSkeleton =>
      'Il n\'y a pas d\'avatar auquel lier le vêtement : exportez sans le lier.';

  @override
  String get motionImport => 'Importer un mouvement capturé…';

  @override
  String get motionFiles => 'Captures de mouvement (BVH)';

  @override
  String errMotionFormat(String detail) {
    return 'Ce fichier BVH est illisible : $detail';
  }

  @override
  String errMotionSkeleton(String bone) {
    return 'Le squelette de ce mouvement n\'a pas d\'articulation pour : $bone.';
  }

  @override
  String errMotionTooShort(String seconds) {
    return 'Ce mouvement est trop court : il faut au moins $seconds s.';
  }

  @override
  String get motionLibrary => 'Bibliothèque de mouvements';

  @override
  String get motionLibraryMenu => 'Bibliothèque de mouvements…';

  @override
  String get motionLibrarySubtitle =>
      'Les mouvements capturés (fichiers BVH) utilisables dans tous vos projets.';

  @override
  String get motionLibraryEmpty =>
      'Aucun mouvement pour l\'instant : ajoutez ici des fichiers BVH, ou ajoutez un mouvement à la bibliothèque en l\'important.';

  @override
  String get motionLibraryAdd => 'Ajouter des fichiers…';

  @override
  String get motionLibraryPlay => 'Jouer sur l\'avatar';

  @override
  String motionLibraryDeleteConfirm(String name) {
    return 'Supprimer « $name » de la bibliothèque ? Les projets qui l\'utilisent gardent leur copie.';
  }

  @override
  String get motionImported => 'Mouvement importé dans le projet.';

  @override
  String get motionAddToLibrary => 'Ajouter à la bibliothèque';

  @override
  String motionAddedToLibrary(String name) {
    return 'Ajouté à la bibliothèque : $name';
  }

  @override
  String errLibraryName(String name) {
    return '« $name » ne peut pas nommer un mouvement : évitez / \\ : * ? \" < > | et un point au début ou à la fin.';
  }

  @override
  String errLibraryExists(String name) {
    return 'La bibliothèque a déjà un mouvement nommé « $name ».';
  }

  @override
  String errLibraryMissing(String name) {
    return 'La bibliothèque n\'a plus de mouvement nommé « $name ».';
  }

  @override
  String errLibraryIo(String detail) {
    return 'La bibliothèque de mouvements ne peut pas être lue ou écrite : $detail';
  }

  @override
  String get barElastic => 'Élastique';

  @override
  String get barElasticHint =>
      'Coudre un élastique le long de ce bord : il fronce au drapé.';

  @override
  String get barShirring => 'Smocks';

  @override
  String get barShirringHint =>
      'Des rangs d\'élastique à travers la pièce, parallèles à ce bord.';

  @override
  String get elasticTitle => 'Élastique';

  @override
  String get elasticRatio => 'Longueur au repos';

  @override
  String get elasticWidth => 'Largeur';

  @override
  String get shirringTitle => 'Smocks';

  @override
  String get shirringRows => 'Rangs';

  @override
  String get shirringFirst => 'Premier rang depuis le bord';

  @override
  String get shirringGap => 'Entre les rangs';

  @override
  String get detailsTitle => 'Détails de montage';

  @override
  String detailElastic(String cm, String width) {
    return 'Élastique · $cm cm, $width mm de large';
  }

  @override
  String get tpElastic => 'Élastique';

  @override
  String get tpElasticBySize => 'Élastique par vêtement et par taille (cm)';

  @override
  String get errHasElastic => 'Ce tronçon a déjà un élastique.';

  @override
  String get barTopstitch => 'Surpiqûre';

  @override
  String get barTopstitchHint =>
      'Des rangs de points le long de ce bord, visibles à l\'endroit.';

  @override
  String get topstitchTitle => 'Surpiqûre';

  @override
  String get topstitchRows => 'Rangs';

  @override
  String get topstitchDistance => 'Depuis le bord';

  @override
  String get topstitchGap => 'Entre les rangs';

  @override
  String get topstitchStitch => 'Longueur de point';

  @override
  String get threadNormal => 'Fil à coudre';

  @override
  String get threadHeavy => 'Fil à surpiquer';

  @override
  String detailTopstitch(int rows, String mm, String stitch) {
    String _temp0 = intl.Intl.pluralLogic(
      rows,
      locale: localeName,
      other: '$rows rangs',
      one: '1 rang',
    );
    return '$_temp0 · à $mm mm du bord, points de $stitch mm';
  }

  @override
  String get errOtherKind => 'C\'est une autre sorte de détail.';

  @override
  String get tpTopstitching => 'Surpiqûres';

  @override
  String get tpRows => 'Rangs';

  @override
  String get tpRowDistance => 'Depuis le bord (mm)';

  @override
  String get tpRowGap => 'Entre les rangs (mm)';

  @override
  String get tpStitch => 'Point (mm)';

  @override
  String get tpSpi => 'Points par pouce';

  @override
  String get tpThread => 'Fil';

  @override
  String get tpThreadNormal => 'normal';

  @override
  String get tpThreadHeavy => 'épais';

  @override
  String get tpElastics => 'Élastiques';

  @override
  String get tpRatio => 'Longueur au repos (%)';

  @override
  String get tpElasticWidth => 'Largeur (mm)';

  @override
  String get tpRelaxed => 'Au repos (cm)';

  @override
  String get barInterfacing => 'Entoilage';

  @override
  String get barInterfacingHint =>
      'Thermocoller un entoilage sur toute la pièce : plus raide, elle tient sa forme.';

  @override
  String get barInterfacingBandHint =>
      'Thermocoller un entoilage en bande le long de ce bord (parementure, ceinture).';

  @override
  String get interfacingTitle => 'Entoilage';

  @override
  String get interfacingWidth => 'Largeur de la bande';

  @override
  String get interfacingLight => 'léger';

  @override
  String get interfacingMedium => 'moyen';

  @override
  String get interfacingHeavy => 'fort';

  @override
  String detailInterfacing(String weight, String cm2) {
    return 'Entoilage $weight · $cm2 cm²';
  }

  @override
  String get errNotClosed => 'Prenez une ligne fermée sur cette pièce.';

  @override
  String get tpInterfacing => 'Entoilage';

  @override
  String get tpInterfacingFabric => 'Sorte';

  @override
  String get tpInterfacingWeight => 'Poids';

  @override
  String get tpInterfacingArea => 'Surface (m²)';

  @override
  String get tpWoven => 'tissé';

  @override
  String get tpNonWoven => 'non tissé';

  @override
  String get tpKnit => 'maille';

  @override
  String get barQuilting => 'Matelassage';

  @override
  String get barQuiltingHint =>
      'Matelasser la pièce sur un rembourrage : elle gonfle entre les piqûres.';

  @override
  String get quiltingTitle => 'Matelassage';

  @override
  String get quiltingSpacing => 'Entre les piqûres';

  @override
  String get quiltingLoft => 'Rembourrage';

  @override
  String get quiltingDiamonds => 'losanges';

  @override
  String get quiltingChannels => 'canaux';

  @override
  String detailQuilting(String pattern, String cm, String mm) {
    return 'Matelassé en $pattern · tous les $cm cm, rembourrage $mm mm';
  }

  @override
  String get tpPadding => 'Rembourrage';

  @override
  String get tpLoft => 'Épaisseur (mm)';

  @override
  String get tpPaddingWeight => 'Grammage (g/m²)';

  @override
  String get tpQuilting => 'Matelassage';

  @override
  String get tpPatternKind => 'Motif';

  @override
  String get tpAngle => 'Angle (°)';

  @override
  String get tpSpacing => 'Écart (mm)';

  @override
  String get templateJeans => 'Jean';

  @override
  String get templateJeansBlurb =>
      'En denim, cinq poches, un empiècement dos, une braguette et des passants.';

  @override
  String get templateShorts => 'Short';

  @override
  String get templateShortsBlurb =>
      'La base du pantalon, arrêtée une main au-dessus du genou.';

  @override
  String get libraryOpen => 'Depuis la bibliothèque…';

  @override
  String get libraryManage => 'Bibliothèque';

  @override
  String get libraryManageSubtitle =>
      'Vos propres patrons : renommez-les, exportez-les, supprimez-les ou importez-en un. Enregistrez un vêtement ici depuis sa carte dans un projet, et ajoutez un patron à un projet depuis celui-ci.';

  @override
  String get libraryTitle => 'Bibliothèque de patrons';

  @override
  String get librarySubtitle =>
      'Cliquez sur un patron pour l\'ajouter à ce projet, tracé pour son corps. Enregistrez ici vos propres vêtements depuis leur carte.';

  @override
  String get libraryKindTops => 'Hauts';

  @override
  String get libraryKindBottoms => 'Bas';

  @override
  String get libraryKindDresses => 'Robes et jupes';

  @override
  String get libraryKindAccessories => 'Accessoires';

  @override
  String get libraryMine => 'Mes patrons';

  @override
  String get libraryMineEmpty =>
      'Aucun pour l\'instant : enregistrez un vêtement depuis sa carte (le bouton signet) pour l\'ajouter à d\'autres projets.';

  @override
  String get libraryImport => 'Importer…';

  @override
  String get libraryExport => 'Exporter…';

  @override
  String get libraryPatternFiles => 'Fichiers de patron';

  @override
  String libraryDeleteConfirm(String name) {
    return 'Supprimer « $name » de la bibliothèque ? Les projets où il a été ajouté gardent leurs vêtements.';
  }

  @override
  String libraryReplaceConfirm(String name) {
    return 'La bibliothèque a déjà un patron nommé « $name ». Le remplacer ?';
  }

  @override
  String get libraryReplace => 'Remplacer';

  @override
  String get librarySave => 'Enregistrer dans la bibliothèque';

  @override
  String get librarySaveTitle => 'Enregistrer dans la bibliothèque de patrons';

  @override
  String get librarySaveName => 'Nom';

  @override
  String get librarySaveKind => 'Type';

  @override
  String librarySaved(String name) {
    return 'Enregistré dans la bibliothèque sous « $name ».';
  }

  @override
  String get busyLibraryAdd => 'Ajout du patron';

  @override
  String get errLibraryNone =>
      'Aucune bibliothèque de patrons n\'est configurée.';

  @override
  String errPatternExists(String name) {
    return 'La bibliothèque a déjà un patron nommé « $name ».';
  }

  @override
  String errPatternMissing(String name) {
    return 'La bibliothèque n\'a plus de patron nommé « $name ».';
  }

  @override
  String get templateSweatshirt => 'Sweat';

  @override
  String get templateSweatshirtBlurb =>
      'Le sweat à capuche sans sa capuche : col rond, froncé aux poignets et en bas.';

  @override
  String get templateBodysuit => 'Body bébé';

  @override
  String get templateBodysuitBlurb =>
      'En jersey, manches courtes, fermé sous l’entrejambe, avec ses tours de cuisse.';

  @override
  String get templatePyjama => 'Pyjama';

  @override
  String get templatePyjamaBlurb =>
      'Haut et bas en jersey : deux vêtements, le haut porté sur le bas.';

  @override
  String get templatePyjamaTop => 'Haut de pyjama';

  @override
  String get templatePyjamaTopBlurb =>
      'Ample, manches longues froncées aux poignets.';

  @override
  String get templatePyjamaBottoms => 'Bas de pyjama';

  @override
  String get templatePyjamaBottomsBlurb =>
      'Ample, à enfiler : taille élastique, ouvert aux chevilles.';

  @override
  String get templateJoggingSuit => 'Ensemble de jogging';

  @override
  String get templateJoggingSuitBlurb =>
      'Un jogging et un sweat porté par-dessus : deux vêtements.';

  @override
  String get bodyAdult => 'Adulte';

  @override
  String get bodyChild => 'Enfant';

  @override
  String get bodyAge => 'Âge';

  @override
  String get bodyAgeYears => 'ans';

  @override
  String get bodyGirl => 'Fille';

  @override
  String get bodyBoy => 'Garçon';

  @override
  String get chartGirls => 'Fille';

  @override
  String get chartBoys => 'Garçon';

  @override
  String get chartBabies => 'Bébé';

  @override
  String get libraryKindChildren => 'Bébés et enfants';

  @override
  String presetGirls(String size) {
    return 'Fille $size ans';
  }

  @override
  String presetBoys(String size) {
    return 'Garçon $size ans';
  }

  @override
  String presetBaby(String months) {
    return 'Bébé $months mois';
  }
}
