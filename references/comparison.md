# Abgleich mit WoT Numbers

Geprüft am 8. November 2026 anhand der offiziellen Dokumentation und der darin enthaltenen Screenshots. Die Anpassung ist eine eigenständige Rust-Implementierung; keine C#-Komponenten und keine kopierten UI-Assets.

| Original | Umsetzung in WoT Insights 0.2 |
|---|---|
| Home / Tanks / Battles | Übersicht / Panzer / Gefechte als Hauptansichten |
| Classic Home mit Kennzahlen und Instrumenten | Kennzahlkarten sowie Anzeigen für Winrate, Schaden/erhaltenen Schaden und Abschüsse pro Gefecht |
| Verteilung nach Stufe und Klasse | Gefechtsverteilungen auf der Übersicht |
| Zeitraumfilter | Heute, 7/30/90 Tage, gesamter Zeitraum und letzte Session |
| Favourite Tank Lists | Lokale Favoritenmarkierung und gemeinsamer Favoritenfilter; noch keine benannten Mehrfachlisten |
| Anpassbare Gefechtsansichten | Sichtbare Zusatzspalten wählbar; Spaltenköpfe sortieren; Einstellungen bleiben erhalten |
| Tank-Tabelle | Aggregation pro Panzer im aktuellen Filter, Favoriten, Sortierung und Sprung zu den Gefechten |
| Charts | Tagesverläufe für Winrate, Schaden, Assist, Basis-XP und Gefechtszahl mit Punktdetails |
| Erfassungsstatus | Aktiver/pausierter Import, letzter erfolgreicher Scan und sichtbare Fehler |
| Maps | Bestehende Karte-×-Klasse-Matrix und Karten-Rangliste bleiben erhalten |

Absichtliche Unterschiede: Die App zeigt nur tatsächlich importierte Replay-Daten. Gesamtwerte des Accounts, WN8 oder Effizienz werden nicht nachgebildet, solange die notwendigen Datenquellen fehlen. Das Original konnte Dossiers und Gefechtsergebnis-Dateien erfassen; der Rust-Prototyp liest derzeit die JSON-Blöcke der Replays. Die aktuelle Erfassung ist deshalb kein vollständiger Ersatz für den ursprünglichen Listener.

Quellen:

- [Navigation und Erfassung](https://wotnumbers.com/ug/2/)
- [Home und Classic-Ansicht](https://wotnumbers.com/ug/3/)
- [Panzer- und Gefechtsspalten](https://wotnumbers.com/ug/4/)
- [Verlaufsauswertung](https://wotnumbers.com/ug/8/)

Die lokal abgelegten Original-Screenshots dienen ausschließlich dem Entwicklungsvergleich und sind nicht Teil der portablen Anwendung.
