# WoT Battle Analyzer

Erstellt von **Friend of Misery**.

Eigenständiger WoT-Statistiktracker in Rust für Windows, angelehnt an die Ansichten des ursprünglichen WoT Numbers. Die Kartenanalyse bleibt ein Schwerpunkt: **Auf welchen Karten spiele ich mit welcher Panzerklasse gut?**

## Version 0.6.3: Kräftigere Farben

Sattere Grün-, Rot- und Blautöne für Kennzahlen, Statusanzeigen und Links. Farbige Texte werden stärker hervorgehoben; die Gefechtsdetails verwenden dieselbe Palette wie die Hauptansichten.

## Version 0.6.2: Lesbarkeit

Unter Windows verwendet die Oberfläche die installierte Segoe UI mit 17 Punkten als Standard, größeren Diagrammbeschriftungen und höherem Textkontrast. Unter **Einstellungen → Darstellung → Schriftgröße** kann die Größe von 14 bis 22 Punkten eingestellt werden; die Auswahl bleibt gespeichert. Falls Segoe UI fehlt, werden die mitgelieferten egui-Schriften verwendet.

## Version 0.6.1: Name und Info

Die Anwendung heißt **WoT Battle Analyzer**. Der dauerhaft sichtbare Reiter **Info** zeigt Version, Ersteller und Kurzbeschreibung. Der GitHub-Link wird nach Veröffentlichung ergänzt. Für Builds kann die bestätigte Repository-Adresse über `WOT_BATTLE_ANALYZER_GITHUB_URL` gesetzt werden. Bestehende Daten werden über den bisherigen Speicherort weitergefunden.

## Version 0.6: Datenmodell, Archiv und Sprachen

- Ein gemeinsames relationales Gefechtsmodell für Statistik, Schaden und Spielersuche; alte deutsche Tabellennamen bleiben als SQL-Ansichten verfügbar.
- Unveränderte Scans laden keine Gefechtsdatensätze erneut. Fahrzeugkatalog und Datenbankverbindung werden zwischengespeichert.
- Vollständige Replay-Binärdaten liegen in einer separaten `battles.archive.db` neben `battles.db`. **Für Sicherungen und Umzüge beide Dateien gemeinsam behandeln.** Die erste Übernahme kopiert und prüft das Archiv und kann mehrere Minuten dauern; alte Programmversionen danach nicht parallel starten.
- Unter **Einstellungen → Sprache** stehen Deutsch, Englisch, Spanisch, Polnisch, Französisch, Tschechisch, Ukrainisch, Russisch, Italienisch, Türkisch, Portugiesisch, Ungarisch und Rumänisch bereit. Hauptbedienung und Kennzahlen sind übersetzt; ausführliche Hilfetexte verwenden teilweise Englisch als Fallback. Einzelne ältere technische Felderklärungen sind noch deutsch.
- Technische Details und SQL-Beispiele: [Architektur](docs/architecture.md).

## SQL-Abfragen

Unter **Einstellungen → SQL-Reiter für eigene Abfragen anzeigen** lässt sich der zusätzliche Reiter **SQL** einschalten. Die Auswahl bleibt gespeichert. Dort können lesende SQL-Abfragen gegen die aktuelle Datenbank ausgeführt werden. Es wird die erste Anweisung ausgewertet, maximal 1.000 Ergebniszeilen angezeigt und nach fünf Sekunden abgebrochen. Lange Textzellen werden nach 2.000 Zeichen gekürzt, BLOBs als Größenangabe dargestellt.

**Tabellen & Spaltendefinitionen** zeigt Tabellen und Views mit Spaltennamen, Datentyp, Pflichtfeld, Standardwert, Primärschlüsselposition, versteckten Spalten und CREATE-Anweisung. Über **Schema aktualisieren** wird die Anzeige neu geladen; je Tabelle gibt es eine SELECT-Vorlage. Gefechtswerte sind direkt als Spalten in `battles` abfragbar. Für bestehende JSON-Abfragen gibt es die Ansicht `battle_json`. Die normalen Ansichtsfilter gelten im SQL-Reiter nicht.

## Schaden und Spielersuche

Die Reiter **Schaden** und **Spielersuche** sind dauerhaft sichtbar und benötigen keinen aktivierten SQL-Reiter.

**Schaden** bietet eingehende und ausgehende Schadenssummen je Gefecht und Gegenfahrzeug mit Datum, Karte, Spieler, Account-ID, Alias, Panzer und Stufe. Zusätzlich werden belegte Trefferzahlen und der gesamte Gefechtsschaden des anderen Spielers angezeigt. Die Munition bleibt unbekannt, solange sie nicht eindeutig je Schadensbeziehung belegt ist.

**Spielersuche** sucht ohne SQL nach Accountnamen, historischen Namen, Aliasnamen oder einer Account-ID. Über die Account-ID werden sämtliche gefundenen Begegnungen dieses Spielers verbunden. **Zusammen** und **Gegeneinander** filtern die Teamzuordnung. Je Begegnung erscheinen Gesamtschaden, Schüsse, direkte Gegnertreffer, Durchschläge sowie Assist durch Aufklärung, Kettenbeschädigung und Betäubung. Diese Werte stammen aus dem gesamten Abschlussbericht des jeweiligen Spielers, nicht nur aus seinem direkten Austausch mit dir. Bei mehreren Fahrzeugdatensätzen werden die bekannten Werte addiert; fehlt ein Teilwert, bleibt die Summe unbekannt. Verwendete Ergebnisfelder: `damageDealt`, `shots`, `directEnemyHits`, `piercingEnemyHits`, `damageAssistedRadio`, `damageAssistedTrack`, `damageAssistedStun`.

Ein Klick auf einen Spieler grenzt die Liste auf dessen Account ein. Ein Klick auf das Datum öffnet die vorhandenen Gefechtsdetails. Beide Reiter bieten Seiten zu 100 Einträgen und eigene Such-/Teamfilter; die Statistikfilter der anderen Reiter gelten hier nicht. Fehlende Angaben erscheinen als **–**, bekannte Nullen als **0**.

Nach dem Update einmal **Jetzt einlesen** ausführen. Vorhandene Archive erhalten dabei die zusätzlichen Teilnehmerwerte auch dann, wenn ihre Originaldateien bereits gelöscht wurden. Die Schemaerweiterung behält vorhandene Daten bei.

## Änderungen

### Version 0.5: Schüsse, Treffer und Durchschläge

- Unter **Panzer → Schussdaten** stehen Summen und Durchschnitte für abgegebene Schüsse, direkte Gegnertreffer und Durchschläge sowie sortierbare Treffer-/Durchschlagsquoten.
- Die Gefechtsliste bietet das Spaltenpaket **Schüsse / Treffer / Durchschläge**. Details zeigen zusätzlich Sprengwirkungstreffer separat. Die Übersicht fasst die Schussdaten im aktuellen Filter zusammen.
- Trefferquote = Treffer / Schüsse. Durchschlagsquote = Durchschläge / Treffer; zusätzlich wird Durchschläge / Schüsse ausgewiesen. Panzer-/Zeitraumquoten werden aus den Summen berechnet, nicht aus dem Mittel der Gefechtsprozente.
- Quelle sind `shots`, `directEnemyHits`, `piercingEnemyHits`; `explosionHits` wird nicht zu direkten Treffern addiert. Das entspricht den eigenen SHOTS/HITS-Records des installierten WoT-Clients. Andere Felder wie `directHits` oder `piercings` werden nicht stillschweigend als Ersatz benutzt.
- Für Summen/Durchschnitte werden nur Endergebnisse mit allen drei bekannten Zählern verwendet; die Fallzahl ist sichtbar. Ein expliziter Wert 0 bleibt gültig. Ein fehlender Wert oder ein Nenner 0 ergibt „–“. Mehrfachtreffer können Zählerverhältnisse über 100 % ergeben; Werte werden nicht künstlich begrenzt.
- Bestehende Replays werden einmal neu eingelesen; Verdienstwerte, Rollen und Filter bleiben erhalten.

### Version 0.4: Verdienst und Kosten

- Neue Ansicht **Verdienst** mit sortierbarer Panzer-Rangliste: Ø Netto, Netto gesamt, Ø Netto ohne Aufgaben sowie ohne Aufgaben/Reserven. Standardmäßig wird nach Ø Netto ohne Aufgaben verglichen; eine Mindestzahl bekannter Abrechnungen kann gewählt werden.
- **Einnahmen & Kosten** zeigt Basisbetrag, Premium, Reserven, Aufgaben/Ereignisse, sonstige Boni/Korrekturen sowie Ausgaben für Munition, Reparatur und Verbrauchsmaterial.
- Die Panzerliste enthält ebenfalls Ø Netto und Ø Netto ohne Aufgaben. Die Gefechtsliste kann Netto anzeigen. Gefechtsdetails enthalten die einzelne Credits-Abrechnung.
- Bereits gespeicherte Replays werden nach dem Update einmal neu eingelesen. Bestehende Filter, Favoriten und Gefechte bleiben erhalten.

Netto folgt den im Replay gemeldeten Kosten: `credits - autoRepairCost - autoLoadCost[0] - autoEquipCost[0]`. Aufgaben und Reserven sind bereits im Brutto enthalten und werden nicht doppelt addiert. Fehlende Werte bleiben unbekannt; unvollständige Abrechnungen zählen nicht in Gewinn- und Kostenmittelwerte. Der Vergleich ohne Aufgaben/Reserven behält Premium bei. Gold/Anleihen werden separat dargestellt, nicht in Credits umgerechnet.

Die Werte sind die automatische Gefechtsabrechnung laut Replay, keine vollständige Geldbeutelbilanz einschließlich manueller Käufe und historischer Lagerkosten. [Feldprüfung und Grenzen](research/economy/README.md).

### Version 0.3: Panzerfilter

Der kompakte Button **Panzerfilter** bietet Mehrfachauswahl für Nation, Panzerklasse, Tier I–XI und Fahrzeugrolle (Sturm, Unterstützung, Durchbruch, Mehrzweck, Scharfschütze, Aufklärung und Artillerie). Innerhalb einer Gruppe werden die Auswahlen mit „oder“, zwischen Gruppen mit „und“ kombiniert. Beispiel: Deutschland oder USA + Heavy + Tier VIII/IX/X + Sturm. Die Auswahl gilt für die Panzerauswahl und sämtliche Auswertungen und wird gespeichert. „Alle …“ leert die jeweilige Gruppe, „Panzerfilter leeren“ alle vier Gruppen.

Nation, Klasse, Rolle und Stufe sind in Panzerliste und Gefechtsdetails sichtbar. Die Rollen kommen aus den Fahrzeug-Tags der aktuellen Installation, nicht aus einer Schätzung anhand des Namens. Fahrzeuge ohne Rollen-Tag bleiben „Nicht zugeordnet“. Auch bereits importierte Gefechte erhalten diese Metadaten beim nächsten Import, ohne ihre Ergebnisse zu verändern. Bisherige einzelne Klassen-/Stufenfilter werden übernommen.

Version 0.2.1 ergänzt **Panzer suchen …** neben der Auswahl. Die Suche schränkt Auswahl und Auswertung ein; zum Beispiel `T20`, `Strv 103` oder `AMX`. Groß-/Kleinschreibung, Leerzeichen und Trennzeichen werden ignoriert. Mit „Alle Suchtreffer“ lassen sich mehrere passende Panzer gemeinsam auswerten. „Filter zurücksetzen“ leert die Suche.

### Version 0.2: Ansichten

- Hauptansichten **Übersicht, Panzer, Gefechte, Karten und Verlauf**, ergänzt durch Einstellungen.
- Übersicht mit Kennzahlen, Instrumenten für Winrate/Schadensverhältnis/Abschüsse, Gefechten nach Klasse/Stufe und den letzten Spieltagen.
- Sortierbare Panzerliste mit Favoriten; ein Klick auf den Namen öffnet die gefilterten Gefechte.
- Sortierbare Gefechtsliste mit wählbaren Zusatzspalten, Seiten zu je 100 Gefechten und Detailfenster.
- Tagesverläufe mit Punktdetails für Winrate, Schaden, Assist, Basis-XP und Gefechtszahl.
- Zeitraum **Letzte Session**: jüngste Folge von Zufallsgefechten, getrennt durch Pausen von mehr als 60 Minuten. Panzer-/Klassenfilter ändern die Sessiongrenze nicht.
- Favoriten, Filter, Ansicht, Spalten und Datenquellen bleiben in `settings.json` neben der Datenbank erhalten. Der Status zeigt Importaktivität und letzte erfolgreiche Aktualisierung.
- Der [Vergleich mit dem Original](references/comparison.md) dokumentiert, welche Funktionen übernommen wurden und wo die Erfassung noch abweicht.

## Start

### Alle Gefechtsdaten mit Erklärungen

Die Gefechtsdetails sind in vier Reiter gegliedert: **Übersicht** mit Kennzahlkarten und Credits-Bilanz, **Eigene Werte** mit thematischen Gruppen, **Gefecht & Teilnehmer** sowie **Originaldaten**. Deutsche Bezeichnungen stehen im Vordergrund. Erklärungen und technische Feldnamen lassen sich getrennt einblenden. Alle vollständigen Originaldaten bleiben erreichbar.

Ein Klick auf das Datum in **Gefechte** öffnet die Details. Die bisherige Zusammenfassung und Credits-Abrechnung ist einklappbar. Darunter zeigt **Vollständige Replaydaten mit Bedeutung** die eigenen Ergebnisdaten und sämtliche originalen JSON-Blöcke, einschließlich Teilnehmern, Gegnerdetails, Zusatzblöcken, leeren Feldern und unbekannten Schlüsseln. Jedes Feld enthält seinen Originalnamen, Wert und eine deutsche Erklärung beziehungsweise einen ausdrücklichen Hinweis auf noch ungeprüfte Bedeutungen. Die Suche berücksichtigt Feldnamen, Werte und Erklärungen. Die JSON-Daten lassen sich vollständig kopieren.

Die Zuordnung zum Gefecht wird anhand der vollständigen Gefechtsidentität geprüft. Gespeicherte Archive funktionieren auch nach Löschung der Originaldatei; ältere Archive werden bei Bedarf zugeordnet. Der binäre Gefechtsverlauf ist als paginierte Hexansicht vollständig zugänglich, aber noch nicht vollständig entschlüsselt oder in Ereignisse übersetzt. Technische IDs und ungeprüfte Codes werden nicht mit erfundenen Namen versehen.

### Vollständiges Replay-Archiv und Bereinigung

Beim nächsten Import werden vorhandene Replays einmal neu eingelesen. Sämtliche JSON-Blöcke (auch unbekannte Felder) und alle übrigen Binärdaten werden verlustfrei in `replay_archives` in der Datenbank gespeichert. Die JSON-Blöcke sind einzeln komprimiert; der bereits komprimierte binäre Verlauf bleibt unverändert erhalten. Noch unbekannte oder unvollständige Formate werden als vollständige Originalbytes archiviert. Die Statistikansichten werten weiterhin nur die bereits unterstützten Felder aus; vollständige Speicherung bedeutet noch keine vollständige Dekodierung aller Ereignisse.

Unter **Replay-Archiv** lassen sich Originaldateien suchen, deren Gefechtsdatum mehr als 30 Tage zurückliegt. Nach Vorschau und Auswahl werden die Dateien zuerst archiviert, aus dem gespeicherten Archiv bytegenau rekonstruiert und geprüft und erst danach endgültig gelöscht. Fehlerhafte oder seit der Vorschau geänderte Dateien bleiben erhalten. Die Statistikdaten bleiben bestehen. Unterordner werden berücksichtigt; Verknüpfungen, `temp.wotreplay` und `replay_last_battle.wotreplay` werden nicht bereinigt. Es gibt keine automatische Löschung beim Start.

Das vollständige Archiv benötigt annähernd so viel Platz wie die ursprünglichen Replays. Solange die Originaldateien noch vorhanden sind, wird dieser Platz zusätzlich benötigt. Die Bereinigung entfernt die Originalkopien, nicht die archivierten Daten.

`cargo run` öffnet die native egui-Oberfläche. `cargo run -- --import` importiert ohne Oberfläche und schreibt eine kurze Auswertung ins Terminal.

Die portable Anwendung liegt nach `cargo build --release` unter `target/release/wot-insights.exe`.

Standardmäßig liest die Anwendung `C:\games\World_of_Tanks_EU\replays` und die Fahrzeuglisten aus `res/packages/scripts.pkg`. Der Replay-Ordner ist unter **Datenquelle & Hinweise** änderbar. Umgebungsvariablen für andere Installationen:

```powershell
$env:WOT_GAME_DIR = 'D:\Games\World_of_Tanks_EU'
$env:WOT_REPLAY_DIR = 'D:\Replays'
$env:WOT_INSIGHTS_DB = 'D:\WoT-Daten\battles.db'
cargo run
```

Die Datenbank liegt in `%LOCALAPPDATA%\WoTInsights\battles.db`. Spiel- und Replay-Dateien werden ausschließlich gelesen. Kein XVM, kein Login, keine Cloud und keine Netzwerkabfragen zur Laufzeit.

## Enthalten

- Inkrementeller Replay-Import mit SQLite, automatische Prüfung alle 30 Sekunden im Hintergrund.
- Aktualisierung später vervollständigter Replays und Erkennung kopierter Gefechte.
- Karte × Klasse: Light, Medium, Heavy, TD und Arty; Winrate, Durchschnittsschaden und Assist.
- Vergleich mit deinem Klassenmittel innerhalb der aktuellen Filter. Grün: mindestens Klassenmittel; Orange: darunter.
- Karten-Rangliste und einzelne Gefechte mit Schaden, Assist, Basis-XP und Brutto-Credits.
- Spieler-, Panzer-, Klassen-, Stufen-, Karten- und Zeitraumfilter.
- Mindestzahl bekannter Endergebnisse; kleine Stichproben bleiben grau. Standard: 10. Auch größere Stichproben garantieren keinen belastbaren Unterschied.
- Fahrzeugklasse und Stufe aus dem aktuellen Client.

## Grenzen der Version 0.5

Diese Version nutzt die unverschlüsselten JSON-Blöcke der `.wotreplay`-Dateien. Bei vorzeitigem Verlassen kann das Endergebnis fehlen. Solche Gefechte bleiben in der Liste, zählen aber nicht als Niederlage oder 0 Schaden. Ein zweiter Import kann ein vervollständigtes Ergebnis ergänzen. Unbekannte Formate werden mit Dateinamen gemeldet und beim nächsten Scan erneut geprüft.

Die vorhandene Datei `replay_last_battle.wotreplay` mit Blockanzahl 0 ist noch nicht unterstützt. Die übrigen datierten Replays sind die Importquelle. Der `battle_results`-Cache wird noch nicht eingelesen. Vollständige automatische Erfassung vorzeitig verlassener Gefechte erfordert als nächsten Schritt einen aktuellen Cache-Parser oder einen kleinen Export-Mod.

Nur `battleType=1` (Zufallsgefechte) wird ausgewertet. Spezialmodi mit mehreren Fahrzeugen werden nicht mit normalen Gefechten vermischt. Assist ist die Summe aus Aufklärungs-, Ketten- und Betäubungsassist, **keine Gunmark-Berechnung**. Brutto, Kosten und Netto sind getrennt beschriftet. Historische Klassen/Stufen können von den heutigen Clientdaten abweichen.

Noch nicht enthalten: WN8, Dossier-Import, manuell definierte Sessions, frei konfigurierbare Widgets, benannte Favoritenlisten, Kartenbilder/Startseitenanalyse, Gunmark-Liveanzeige und 3D-Trefferanalyse. Verläufe sind Tageswerte der importierten Gefechte, keine kumulierten Kontostatistiken. Die Rangliste beschreibt deine gespeicherten Gefechte; sie bereinigt nicht um Panzer-/Stufenverteilung. Das Schadensverhältnis verwendet nur Gefechte mit beiden Werten; bei Nenner 0 wird kein Zahlenwert angezeigt.

## Prüfung

```powershell
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
```

Tests: fehlende Endergebnisse, Teamzuordnung, Mehrfahrzeug-Ergebnisse, unverfälschte Nenner und Schadensverhältnisse, Duplikate, spätere Ergebnisergänzungen, beschädigte Dateien, Sessiongrenzen, gemeinsame Zeitraum-/Favoritenfilter, gespeicherte Einstellungen und Render-Durchläufe aller sechs Ansichten bei 1000×620 und 1280×820. Render-Tests prüfen den UI-Code ohne Fenstersteuerung; sie ersetzen keine visuelle Abnahme.

Eigenständige Implementierung; kein Quellcode von WoT Numbers übernommen. Referenz für das Replay-Containerformat: [kosh04/wotreplay](https://github.com/kosh04/wotreplay). World of Tanks ist eine Marke von Wargaming; dieses Projekt ist unabhängig.

## Spieler, Gefechte und Schaden per SQL

Beim nächsten **Jetzt einlesen** werden auch bereits vorhandene Datenbankarchive aufbereitet, selbst wenn die Original-Replays gelöscht wurden. Der installierte Fahrzeugkatalog ergänzt die Stufen. Im SQL-Reiter anschließend **Schema aktualisieren** wählen.

- `spieler`: eindeutige Account-ID und zuletzt eingelesener echter Accountname. Eine Namensänderung erzeugt keinen zweiten Spieler.
- `gefechte`: Gefechtskennung, Datum, Karte und eigener Spieler.
- `gefecht_teilnehmer`: Zuordnung zwischen Gefecht, Fahrzeug und Account. Name, angezeigter Name und abweichender Alias bleiben als historischer Stand des jeweiligen Gefechts erhalten; hinzu kommen Panzer, Stufe und Team. Ohne belegte Account-ID wird keine Identität aus dem Namen geraten.
- `schaden_eingehend` / `schaden_ausgehend`: eine Zeile je Gefecht, eigenem Fahrzeugdatensatz, Gegenfahrzeug und dessen Lebensindex, sofern positive Schadens-/Trefferwerte vorliegen (eingehend auch Blockschaden oder bestätigter Zerstörer). Keine Einzelgeschoss- oder Zeitverlaufsdaten. Die Tabellen verweisen über Gefechts- und Fahrzeugkennung auf die Teilnehmer.
- `v_spieler_gefechte`, `v_schaden_eingehend`, `v_schaden_ausgehend`: fertig verknüpfte Ansichten mit Namen, Alias, Panzer, Stufe, Karte und Teamzuordnung. In den Schadensansichten bezeichnet der Teilnehmer den Angreifer bzw. das Ziel; `eigener_spieler_id` steht in `gefechte` bzw. `v_spieler_gefechte`.
- `replay_json`: sämtliche lesbaren JSON-Blöcke ohne Feldauswahl, je Archivpfad und Blocknummer. Die vollständigen Originalbytes einschließlich Binärstream bleiben in `replay_archives` erhalten.

Munitionstypen sind in den ausgewerteten Ergebnisdetails nicht je Gegenfahrzeug enthalten und bleiben `NULL`. Eingehende Durchschlags- und Explosionstreffer bleiben ebenfalls `NULL`, weil entsprechende paarweise Zähler dort fehlen. Ausgehend werden `piercings` und `explosionHits` separat übernommen. Explosionstreffer werden nicht als Beweis für HE-Munition interpretiert. Fehlende Zahlen werden nicht in 0 umgewandelt. Die Stufe stammt aus dem aktuell installierten Fahrzeugkatalog.

Alle Gefechte eines Spielers (über seine Account-ID, auch bei wechselnden Aliasnamen):

```sql
SELECT datum, karte, echter_name, alias, panzer, stufe, zuordnung
FROM v_spieler_gefechte
WHERE spieler_id = 123456789
ORDER BY datum DESC;
```

Spieler anhand eines Namens oder Alias finden:

```sql
SELECT DISTINCT spieler_id, echter_name, angezeigter_name, alias
FROM v_spieler_gefechte
WHERE echter_name LIKE '%Name%'
   OR angezeigter_name LIKE '%Name%'
   OR alias LIKE '%Name%';
```

Eingehender Schaden:

```sql
SELECT datum, karte, echter_name, alias, panzer, stufe,
       schaden, direkte_treffer, geblockter_schaden, zerstoert_von
FROM v_schaden_eingehend
ORDER BY datum DESC, schaden DESC;
```

Ausgehender Schaden mit Durchschlags- und Explosionstreffern:

```sql
SELECT datum, karte, echter_name, alias, panzer, stufe,
       schaden, durchschlagstreffer, explosionstreffer, munitionstyp
FROM v_schaden_ausgehend
ORDER BY datum DESC, schaden DESC;
```
