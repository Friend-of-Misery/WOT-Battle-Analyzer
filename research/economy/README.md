# Credits: geprüfte Felder

Am 10. November 2026 gegen die im installierten EU-Client enthaltenen Battle-Results-Records und lokale Replays geprüft. Die kompilierten Dateien wurden ausschließlich als Daten disassembliert, nicht ausgeführt. Keine Übernahme von Client-Code in die Anwendung.

## Quelle im installierten Client

`res/packages/scripts.pkg` enthält:

- `scripts/client/gui/battle_results/presenters/packers/economics/credits_records.pyc`: `TOTAL_CREDITS` nutzt die Credits und zieht `autoRepairCost`, `autoLoadCredits` und `autoEquipCredits` ab.
- `scripts/client/gui/battle_results/reusable/economics_records.pyc`: zusätzliche Kosten werden aus `autoRepairCost`, `autoLoadCost[0]` (Credits), `[1]` (Gold), `autoEquipCost[0]` (Credits), `[1]` (Gold), `[2]` (Anleihen) gebildet.
- `scripts/client/gui/battle_results/presenters/packers/economics/value_extractors.pyc`: die Gefechtsanzeige führt Brutto und die zusätzlichen Abzüge zusammen.

## Verarbeitung

- `credits`: gesamter gemeldeter Bruttobetrag. In allen 2.220 beim ersten Prüflauf lesbaren zufälligen Einzel-Fahrzeug-Ergebnissen galt `credits = factualCredits + eventCredits`. **Aufgabenbeträge werden deshalb nicht erneut addiert.**
- `originalCredits`: Basisbetrag laut Replay; kann Fahrzeugfaktoren und Gefechtsauszeichnungen enthalten. Kein garantierter rein leistungsbezogener Anteil.
- Premiumbonus: `subtotalCredits - originalCredits` nur dann separat ausgewiesen, wenn `subtotalCredits` dem gerundeten Basisbetrag × `appliedPremiumCreditsFactor100 / 100` entspricht. Andernfalls unbekannt.
- `boosterCredits`, `orderCredits`, `eventCredits`: gesondert gespeicherte Reserven-/Aufgabenbeträge. Fehlende Felder bleiben unbekannt.
- Sonstige Boni/Korrekturen: Differenz zwischen Brutto und den bekannten Einzelkomponenten. Dadurch werden z. B. Zugboni nicht fälschlich einer Reserve zugeordnet. Der Rest wird nur berechnet, wenn alle für die Zerlegung benötigten Felder vorliegen.
- Netto: `credits - autoRepairCost - autoLoadCost[0] - autoEquipCost[0]`. Keine Berechnung, wenn ein erforderlicher Wert fehlt; explizite Null ist gültig.
- Ohne Aufgaben: Netto minus `eventCredits`.
- Ohne Aufgaben/Reserven: zusätzlich minus `boosterCredits` und `orderCredits`. Premium bleibt enthalten. Das sind bereinigte Vergleichswerte, keine weiteren Einnahmen.
- Gold und Anleihen werden separat gezeigt und nicht in Credits umgerechnet.

## Grenze

Die Werte bilden die im Replay ausgewiesene automatische Gefechtsabrechnung ab. Außerhalb des Gefechts gekaufte Munition, Lagerbestände, historische Einkaufspreise, Rabatte und manuelle Käufe ergeben daraus keine vollständige Geldbeutel- oder Ressourcenbilanz. Es wird kein hypothetischer Wiederbeschaffungswert geschätzt.

In Summen und Kostenmittelwerte gehen nur Gefechte mit vollständig bekannter Nettoabrechnung ein. Zusätzliche Vergleichswerte ohne Aufgaben/Reserven benötigen auch die jeweiligen Bonusfelder. Angezeigte Fallzahlen machen die Datenabdeckung sichtbar.
