# WoT Battle Analyzer

Ein Statistiktracker für World of Tanks unter Windows, erstellt von **Friend of Misery**. Wertet lokale Replays aus und zeigt, auf welchen Karten und mit welchen Panzern du erfolgreich spielst.

## Funktionen

- Übersicht mit Winrate, Schaden, Assist und Gefechtsverlauf.
- Auswertungen nach Panzer, Klasse, Stufe, Karte und Zeitraum.
- Gefechtsdetails, Credits-Bilanz, Schussdaten und Spielersuche.
- Automatischer Replay-Import, lokales Archiv und eigene SQL-Abfragen.
- Oberfläche in 13 Sprachen.

## Starten

Mit installiertem Rust:

```powershell
cargo run --release
```

Unter **Einstellungen** den Spiel- und Replay-Ordner auswählen und **Jetzt einlesen** starten. Im Spiel die Aufzeichnung aller Gefechte aktivieren.

Die Auswertung nutzt Zufallsgefechte aus `.wotreplay`-Dateien. Fehlende Endergebnisse werden als unbekannt behandelt. Alle Daten bleiben lokal; ein Login ist nicht erforderlich.

## Entwicklung

```powershell
cargo test
cargo build --release
```

Technische Details: [Architektur](docs/architecture.md).

Unabhängiges Projekt. World of Tanks ist eine Marke von Wargaming.
