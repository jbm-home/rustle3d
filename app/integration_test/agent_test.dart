// Attached agents: allowed from the app, an agent connects with the token,
// its change shows at once; the user disconnects it.

import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:rustle3d/main.dart';
import 'package:rustle3d/src/projects.dart';
import 'package:rustle3d/src/rust/api/engine.dart';
import 'package:rustle3d/src/rust/frb_generated.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async => await RustLib.init());

  testWidgets('an agent edits the open project live', (tester) async {
    final dir = Directory.systemTemp.createTempSync('r3d').path;
    final store = ProjectStore(projectsDir: dir, libraryDir: dir);
    File('$dir/tour-seen').createSync();
    final engine = Engine.open(
      path: store.create(
        name: 'Skirt',
        body: store.standardBody(),
        template: 'a_line_skirt',
      ),
    );
    await tester.pumpWidget(Rustle3dApp(store: store, engine: engine));
    Future<void> frames([int n = 20]) async {
      for (var i = 0; i < n; i++) {
        await tester.pump(const Duration(milliseconds: 50));
      }
    }

    await frames();
    expect(find.textContaining('3 pieces'), findsOneWidget);

    engine.allowAgents(libraryDir: dir, allow: true);
    await frames();
    expect(find.text('Agents: waiting'), findsOneWidget);
    final endpoint = jsonDecode(File('$dir/agent.json').readAsStringSync());

    // The agent: hello with the token, then MCP over newline JSON.
    final socket = await Socket.connect('127.0.0.1', endpoint['port'] as int);
    final replies = StreamIterator(
      socket
          .cast<List<int>>()
          .transform(utf8.decoder)
          .transform(const LineSplitter()),
    );
    Future<Map<String, dynamic>> next() async {
      await replies.moveNext().timeout(const Duration(seconds: 10));
      return jsonDecode(replies.current) as Map<String, dynamic>;
    }

    void send(Map<String, Object?> message) =>
        socket.write('${jsonEncode(message)}\n');

    socket.write('R3D-ATTACH ${endpoint['token']}\n');
    send({
      'jsonrpc': '2.0',
      'id': 1,
      'method': 'initialize',
      'params': {
        'protocolVersion': '2025-06-18',
        'capabilities': {},
        'clientInfo': {'name': 'test', 'version': '0'},
      },
    });
    expect((await next())['result']['serverInfo']['name'], 'rustle3d');
    send({'jsonrpc': '2.0', 'method': 'notifications/initialized'});
    send({
      'jsonrpc': '2.0',
      'id': 2,
      'method': 'tools/call',
      'params': {
        'name': 'panel_create_rect',
        'arguments': {'width': 200, 'height': 300},
      },
    });
    expect((await next())['result']['isError'], false);

    // The app shows it without being asked.
    await frames();
    expect(find.textContaining('4 pieces'), findsOneWidget);
    expect(find.text('Agent connected'), findsOneWidget);

    // The user disconnects it, then stops accepting agents.
    engine.disconnectAgent();
    await frames();
    expect(find.text('Agents: waiting'), findsOneWidget);
    expect(
      await replies.moveNext().timeout(const Duration(seconds: 5)),
      isFalse,
    );
    engine.allowAgents(libraryDir: dir, allow: false);
    await frames();
    expect(find.text('Agents'), findsOneWidget);
    expect(File('$dir/agent.json').existsSync(), isFalse);
    socket.destroy();
  });
}
