//! Complete, unfiltered replay JSON plus the undecoded stream.
use anyhow::{Context, Result, ensure};
use chrono::NaiveDateTime;
use serde_json::Value;
use std::io::{Cursor, Read};

#[derive(Clone, Debug)]
pub struct Document {
    pub source: String,
    pub blocks: Vec<Value>,
    pub stream: Vec<u8>,
    pub original_size: usize,
}

pub fn identity(header: &Value) -> Result<String> {
    let date = NaiveDateTime::parse_from_str(
        header["dateTime"].as_str().context("Gefechtsdatum fehlt")?,
        "%d.%m.%Y %H:%M:%S",
    )?
    .format("%Y-%m-%d %H:%M:%S")
    .to_string();
    Ok(serde_json::to_string(&serde_json::json!([
        date,
        header["playerID"],
        header["playerName"],
        header["playerVehicle"],
        header["mapName"],
        header["battleType"]
    ]))?)
}

pub fn read(bytes: &[u8], source: String) -> Result<Document> {
    let mut input = Cursor::new(bytes);
    let mut word = [0; 4];
    input.read_exact(&mut word)?;
    ensure!(
        u32::from_le_bytes(word) == 0x1134_3212,
        "Ungültige Replay-Signatur"
    );
    input.read_exact(&mut word)?;
    let count = u32::from_le_bytes(word);
    ensure!(count <= 16, "Unbekannte Blockanzahl");
    let mut blocks = Vec::new();
    for _ in 0..count {
        input.read_exact(&mut word)?;
        let size = u32::from_le_bytes(word) as usize;
        ensure!(size <= 16 * 1024 * 1024, "JSON-Block zu groß");
        let mut block = vec![0; size];
        input.read_exact(&mut block)?;
        blocks.push(serde_json::from_slice(&block)?);
    }
    Ok(Document {
        source,
        blocks,
        stream: bytes[input.position() as usize..].to_vec(),
        original_size: bytes.len(),
    })
}

/// Meanings are intentionally conservative: unknown IDs and enums are not guessed.
pub fn explanation(key: &str) -> String {
    if key.parse::<u64>().is_ok() || (key.starts_with('(') && key.ends_with(')')) {
        return "Interne Datensatz-/Objektkennung. Der übergeordnete Bereich legt fest, ob es sich etwa um ein Fahrzeug, einen Spieler oder einen Gegnerdetaildatensatz handelt. Die Unterfelder enthalten die zugehörigen Werte.".into();
    }
    if key.starts_with('[') && key.ends_with(']') {
        return "Position innerhalb der übergeordneten Liste, beginnend bei 0. Die Bedeutung der Liste ist am übergeordneten Feld beschrieben.".into();
    }
    let text = match key {
        "dateTime" => "Datum und Uhrzeit des Gefechts laut Replay.",
        "playerName" | "name" => "Gespeicherter Spieler- oder Objektname.",
        "playerID" | "accountDBID" => "Interne Konto-Kennung; keine Leistungskennzahl.",
        "playerVehicle" | "vehicleType" => {
            "Interne Fahrzeugkennung. Die App ergänzt den lesbaren Panzernamen aus dem Fahrzeugkatalog."
        }
        "mapDisplayName" => "Anzeigename der Karte.",
        "mapName" => "Interne Kennung der Karte.",
        "battleType" | "bonusType" => {
            "Numerischer Gefechtstyp. Unsere Statistik wertet battleType 1 als Zufallsgefecht aus; weitere Codes werden unverändert angezeigt."
        }
        "gameplayID" => "Kennung der Spielvariante auf dieser Karte.",
        "clientVersionFromExe" | "clientVersionFromXml" => {
            "Version des Clients, mit dem das Replay aufgezeichnet wurde."
        }
        "regionCode" => "Region der Spielinstallation beziehungsweise des Servers.",
        "serverName" => "Gespeicherter Name des Spielservers.",
        "serverSettings" => {
            "Servereinstellungen zum Aufzeichnungszeitpunkt. Unterfelder bleiben vollständig sichtbar."
        }
        "hasMods" => {
            "Vom Client gemeldetes Mod-Kennzeichen; keine vollständige Liste installierter Mods."
        }
        "vehicles" => "Fahrzeugdaten der Gefechtsteilnehmer, jeweils unter internen Kennungen.",
        "players" => "Spielerdaten der Gefechtsteilnehmer.",
        "personal" => {
            "Persönliche Endergebnisse. Zahlen als Schlüssel bezeichnen Fahrzeugdatensätze; avatar enthält spielerbezogene Angaben."
        }
        "avatar" | "avatars" => {
            "Spielerbezogene Daten, teilweise zusätzlich zu den Fahrzeugergebnissen."
        }
        "common" => "Allgemeine Angaben zum Gefechtsende.",
        "team" => "Teamnummer des Spielers oder Fahrzeugs.",
        "winnerTeam" => "Siegerteam: 1 oder 2. Der Wert 0 wird als Unentschieden ausgewertet.",
        "duration" => "Gemeldete Gefechtsdauer in Sekunden.",
        "arenaCreateTime" => {
            "Erstellungszeit der Gefechtsarena als Unix-Zeitstempel (Sekunden seit 1970)."
        }
        "arenaUniqueID" => "Eindeutige Kennung der Gefechtsarena.",
        "arenaTypeID" => "Interne Kennung des Arena-/Kartentyps.",
        "finishReason" => {
            "Numerischer Grund des Gefechtsendes. Die Zuordnung der Codes ist hier noch nicht geprüft."
        }
        "finishAllPlayersLeft" => "Kennzeichen zum Gefechtsende nach Verlassen durch alle Spieler.",
        "teamHealth" => {
            "Gespeicherte Lebenspunktedaten der Teams; Unterstruktur und Zeitpunkt hängen vom Format ab."
        }
        "bots" => "Gespeicherte Angaben zu Bots im Gefecht.",
        "battleModifiersDescr" => "Interne Beschreibung aktiver Gefechtsmodifikatoren.",
        "damageDealt" => "Verursachter Schaden an Gegnern in Lebenspunkten.",
        "damageReceived" => "Erhaltener Schaden in Lebenspunkten.",
        "damageBlockedByArmor" => "Vom Spiel als durch Panzerung geblockt gemeldeter Schaden.",
        "potentialDamageReceived" => {
            "Vom Spiel gemeldeter potenziell erhaltener Schaden; nicht mit tatsächlich verlorenen Lebenspunkten gleichsetzen."
        }
        "damageReceivedFromInvisibles" => {
            "Erhaltener Schaden von zum Trefferzeitpunkt nicht sichtbaren Gegnern laut Ergebnisdaten."
        }
        "damageAssistedRadio" => "Schaden anderer Spieler durch eigene Aufklärung.",
        "damageAssistedTrack" => "Schaden anderer Spieler durch eigene Kettenbeschädigung.",
        "damageAssistedStun" => "Schaden anderer Spieler durch eigene Betäubung.",
        "damageAssistedSmoke" => {
            "Modusspezifischer Assist im Zusammenhang mit Rauch. Genaue Anrechnungsregeln hier noch nicht geprüft."
        }
        "damageAssistedInspire" => {
            "Modusspezifischer Assist durch Inspiration. Genaue Anrechnungsregeln hier noch nicht geprüft."
        }
        "kills" => "Anzahl zerstörter gegnerischer Fahrzeuge.",
        "damaged" => "Anzahl beschädigter Fahrzeuge laut Ergebniszähler.",
        "damagedHp" => {
            "Zusätzlicher Schadenszähler des Clients. Die genaue Abgrenzung zu damageDealt ist noch nicht geprüft."
        }
        "shots" => "Abgegebene Schüsse.",
        "directEnemyHits" => {
            "Direkte Gegnertreffer. Diesen Zähler nutzt die App für Treffer / Schüsse."
        }
        "piercingEnemyHits" => {
            "Gegnerdurchschläge. Diesen Zähler nutzt die App für die Durchschlagsquote."
        }
        "explosionHits" => {
            "Gemeldete Sprengwirkungstreffer; werden nicht zu direkten Gegnertreffern addiert."
        }
        "directHits" | "piercings" => {
            "Allgemeiner Treffer-/Durchschlagszähler. Nicht ungeprüft mit dem entsprechenden Gegnertrefferzähler gleichsetzen."
        }
        "directTeamHits" => "Direkte Treffer auf Fahrzeuge des eigenen Teams.",
        "directHitsReceived" => "Erhaltene direkte Treffer.",
        "piercingsReceived" => "Erhaltene Durchschläge.",
        "noDamageDirectHitsReceived" => {
            "Erhaltene direkte Treffer ohne Schaden laut Ergebniszähler."
        }
        "explosionHitsReceived" => "Erhaltene Sprengwirkungstreffer.",
        "rickochetsReceived" => "Vom Replay gemeldete erhaltene Abpraller.",
        "sniperDamageDealt" => {
            "Vom Spiel als Fernkampfschaden gemeldeter Anteil. Die Entfernungsgrenze wird hier nicht zusätzlich interpretiert."
        }
        "damageRamming" => "Schaden durch Rammen im jeweiligen Detaildatensatz.",
        "tdamageDealt" => "Verursachter Teamschaden laut Ergebniszähler.",
        "tkills" => "Zerstörte verbündete Fahrzeuge.",
        "tdestroyedModules" => {
            "Modulschäden an Verbündeten laut Ergebniszähler; genaue Zählweise noch nicht geprüft."
        }
        "spotted" => "Aufgeklärte Gegner laut Gefechtsergebnis.",
        "capturePoints" => "Erzielte Basiseroberungspunkte.",
        "droppedCapturePoints" => "Zurückgesetzte gegnerische Basiseroberungspunkte.",
        "capturingBase" => {
            "Gespeicherte Angabe zur eigenen Basiseroberung; Format je Spielmodus beachten."
        }
        "health" => "Verbleibende Lebenspunkte zum gespeicherten Zeitpunkt.",
        "maxHealth" => "Maximale Lebenspunkte des Fahrzeugs.",
        "lifeTime" => "Gemeldete Lebens-/Einsatzdauer des Fahrzeugs in Sekunden.",
        "mileage" => {
            "Fahrstrecke laut internem Replayzähler. Die Maßeinheit ist hier noch nicht verifiziert."
        }
        "deathCount" => "Anzahl der Fahrzeugtode; in Respawn-Modi können es mehrere sein.",
        "deathReason" => {
            "Numerische Todesursache. Die Codes werden unverändert angezeigt und hier noch nicht übersetzt."
        }
        "killerID" => "Interne Kennung des Spielers/Fahrzeugs, das dieses Fahrzeug zerstört hat.",
        "isAlive" => {
            "Gespeichertes Kennzeichen, ob das Fahrzeug lebt; der Zeitpunkt hängt vom jeweiligen Datenblock ab."
        }
        "committedSuicide" => "Vom Spiel gemeldetes Selbstzerstörungskennzeichen.",
        "isTeamKiller" => "Vom Spiel gemeldetes Teamkillerkennzeichen.",
        "isPrematureLeave" => "Kennzeichen für vorzeitiges Verlassen des Gefechts.",
        "watchedBattleToTheEnd" => "Kennzeichen, ob das Gefecht bis zum Ende verfolgt wurde.",
        "stunNum" => "Anzahl gemeldeter Betäubungen.",
        "stunDuration" => {
            "Gespeicherte Betäubungsdauer; Werte werden in der originalen Einheit angezeigt."
        }
        "stunned" => {
            "Zusätzlicher Betäubungszähler; genaue Abgrenzung zu stunNum noch nicht geprüft."
        }
        "credits" => {
            "Gemeldete Brutto-Credits inklusive der enthaltenen Boni. Automatische Kosten müssen davon abgezogen werden."
        }
        "originalCredits" => {
            "Basisverdienst laut Replay vor separat ausgewiesenen Bonusstufen; kann bereits Fahrzeugfaktoren und Auszeichnungen enthalten."
        }
        "subtotalCredits" => {
            "Credits-Zwischensumme. Zusammen mit dem angewendeten Premiumfaktor prüft die App daraus den Premiumbonus."
        }
        "factualCredits" => {
            "Credits-Zwischensumme vor eventCredits in den geprüften Replays; keine zusätzliche Einnahme neben credits."
        }
        "boosterCredits" => {
            "Zusätzliche Credits aus persönlichen Reserven; bereits in den Brutto-Credits enthalten."
        }
        "orderCredits" => {
            "Zusätzliche Credits aus Clan-/Kampfreserven; bereits im Brutto enthalten."
        }
        "eventCredits" => {
            "Gemeinsam gemeldete Credits aus Aufgaben/Ereignissen; Tagesaufgaben und andere Missionen sind daraus nicht pauschal trennbar."
        }
        "autoRepairCost" => {
            "Automatische Fahrzeugreparatur nach dem Gefecht in Credits; kein Reparaturset im Gefecht."
        }
        "autoLoadCost" => {
            "Automatische Munitionsauffüllung. Arrayposition 0 = Credits, 1 = Gold. Keine Aussage über historische Lager-Einkaufspreise."
        }
        "autoEquipCost" => {
            "Automatische Verbrauchsmaterialauffüllung insgesamt. Arrayposition 0 = Credits, 1 = Gold, 2 = Anleihen. Food/Medikit/Reparaturset sind darin nicht einzeln ausgewiesen."
        }
        "autoEquipBoostersCost" => {
            "Gespeicherte Auffüllkosten für Booster. Struktur und Zuordnung zu den sonstigen Abzügen noch nicht geprüft; nicht zusätzlich zum Netto abziehen."
        }
        "repair" => {
            "Separater interner Reparaturwert; die Nettoabrechnung der App verwendet autoRepairCost, um Doppelzählung zu vermeiden."
        }
        "creditsPenalty" | "originalCreditsPenalty" => {
            "Gemeldete Credits-Strafe beziehungsweise deren Ausgangswert. Nicht ungeprüft ein zweites Mal von credits abziehen."
        }
        "xp" => "Gemeldete Erfahrungspunkte im jeweiligen Ergebnisdatensatz.",
        "originalXP" => {
            "Basis-XP vor den separat berechneten Bonusstufen; diese nutzt die App als Basis-XP."
        }
        "freeXP" => "Gemeldete freie Erfahrungspunkte.",
        "tmenXP" => "Gemeldete Besatzungserfahrung.",
        "xp/assist" => "Interner XP-Anteil für Unterstützung laut Replay.",
        "xp/attack" => "Interner XP-Anteil für Angriff laut Replay.",
        "xp/other" => "Interner sonstiger XP-Anteil laut Replay.",
        "xpPenalty" => {
            "Gemeldete XP-Strafe; nicht ungeprüft erneut von der bereits berechneten XP-Summe abziehen."
        }
        "dailyXPFactor10" => {
            "Tages-/Erstsiegfaktor für XP in Zehnteln: beispielsweise 20 entspricht Faktor 2,0. Das ist kein Creditsbonus."
        }
        "isPremium" => "Gespeichertes Premiumkennzeichen.",
        "premMask" => "Interne Premiummaske. Einzelne Bits werden hier noch nicht interpretiert.",
        "isWoTPlus" => "Gespeichertes WoT-Plus-Kennzeichen.",
        "wotPlusTier" => "Gespeicherte WoT-Plus-Stufe als interner Code.",
        "gold" => "Gemeldeter Goldbetrag. Gold wird nicht in Credits umgerechnet.",
        "crystal" => "Gemeldeter Anleihenbetrag. Anleihen werden nicht in Credits umgerechnet.",
        "equipCoin" => "Gemeldete Ausrüstungskomponenten.",
        "eventCoin" | "bpcoin" => {
            "Modus-/ereignisbezogene Währung. Die genaue Bezeichnung hängt vom Ereignis ab."
        }
        "currencies" => "Zusätzliche Währungen mit ihren internen Kennungen.",
        "achievements" | "inBattleAchievements" => {
            "Auszeichnungen als interne IDs beziehungsweise Datensätze. Die Namen der IDs sind hier noch nicht vollständig zugeordnet."
        }
        "markOfMastery" | "prevMarkOfMastery" => {
            "Aktueller beziehungsweise vorheriger Meisterschaftsabzeichen-Code. Rohwert bleibt sichtbar."
        }
        "marksOnGun" => {
            "Gespeicherter Gunmark-/Geschützmarkierungswert. Noch keine eigenständige Rekonstruktion des Gunmark-Verlaufs."
        }
        "movingAvgDamage" => {
            "Interner gleitender Schadenswert für Markierungen. Nicht mit dem einfachen Schadensmittel der App gleichsetzen."
        }
        "damageRating" => {
            "Interner Schadens-/Markierungswert. Skala und Rundung werden hier noch nicht zusätzlich interpretiert."
        }
        "battlePassPoints" => "Gemeldete Battle-Pass-Punkte aus dem Gefecht.",
        "battlePassComplete" => "Gespeichertes Battle-Pass-Abschlusskennzeichen.",
        "hasBattlePass" => {
            "Gespeichertes Battle-Pass-Kennzeichen; genaue Produkt-/Saisonzuordnung noch nicht geprüft."
        }
        "bpChaptersInfo" => "Battle-Pass-Kapiteldaten und interne Kapitelkennungen.",
        "questsProgress" | "PM2Progress" => {
            "Aufgaben-/Kampagnenfortschritt. Aufgaben- und Bedingungs-IDs bleiben als verschachtelte Daten sichtbar."
        }
        "questTokensCount" | "questTokensConvertion" => {
            "Gespeicherte Aufgabenmarken beziehungsweise deren Umwandlung."
        }
        "dossierLogRecords" | "dossierPopUps" => {
            "Interne Statistik-/Dossier-Einträge beziehungsweise Benachrichtigungen aus diesem Ergebnis."
        }
        "details" => {
            "Detaildaten je beteiligtem Gegner/Objekt. Schlüssel wie (Kennung, Index) sind interne Zuordnungen; Unterfelder zeigen die jeweiligen Begegnungswerte."
        }
        "damageEventList" | "avatarDamageEventList" => {
            "Liste gespeicherter Schadensereignisse. Struktur und Ereigniscodes sind versionsabhängig und noch nicht vollständig dekodiert."
        }
        "typeCompDescr" | "accountCompDescr" => {
            "Kompakte interne Fahrzeug-/Kontobeschreibung. Nicht als lesbare Bezeichnung interpretieren."
        }
        "clanDBID" => "Interne Clan-Kennung.",
        "clanAbbrev" => "Clankürzel.",
        "prebattleID" => "Interne Zuordnung zu Zug beziehungsweise Vorbereitungsgruppe.",
        "setupsIndexes" => {
            "Indizes der ausgewählten Ausstattungsvarianten; keine vollständigen Namen der eingebauten Gegenstände."
        }
        "outfit" => "Gespeicherte Anpassungs-/Tarnungsdaten.",
        "badges" | "overriddenBadge" => {
            "Abzeichenkennungen; die Namen sind hier noch nicht zugeordnet."
        }
        "dogTags" => "Gespeicherte Dogtag-Daten und Kennungen.",
        "position" => {
            "Gespeicherte Position in diesem Datenblock; spezielle Platzhalterwerte sind keine reale Kartenposition."
        }
        "fogOfWar" => {
            "Gespeichertes Kennzeichen zum Nebel des Krieges beziehungsweise zur Sichtinformation."
        }
        "replayURL" => "Im Replay gespeicherter Verweis; wird nur als Text angezeigt.",
        "isFirstBlood" => "Kennzeichen zum ersten Abschuss laut Ergebnisdaten.",
        "winAloneAgainstVehicleCount" => {
            "Gespeicherte Zahl gegnerischer Fahrzeuge beim Alleinsieg."
        }
        "damageBeforeTeamWasDamaged" | "killsBeforeTeamWasDamaged" => {
            "Schaden beziehungsweise Abschüsse, bevor das eigene Team erstmals Schaden erhielt."
        }
        "damageWhileMoving" | "damagedWhileMoving" => {
            "Gespeicherter Schadens-/Ereigniszähler während eigener Bewegung; genaue Zählweise noch nicht geprüft."
        }
        "damagedWhileEnemyMoving" => {
            "Gespeicherter Schadens-/Ereigniszähler gegen bewegte Gegner; genaue Zählweise noch nicht geprüft."
        }
        "percentFromTotalTeamDamage" => {
            "Gespeicherter Anteil am Teamschaden. Interne Skalierung wird unverändert angezeigt."
        }
        "percentFromSecondBestDamage" => {
            "Gespeichertes Verhältnis zum zweitbesten Schadenswert. Interne Skalierung wird unverändert angezeigt."
        }
        "rolloutsCount" => "Anzahl gemeldeter Ausfahrten/Einsätze bei entsprechenden Modi.",
        "stopRespawn" => "Kennzeichen zur Beendigung weiterer Respawns.",
        "commendationsReceived" | "commendationsSent" => {
            "Erhaltene beziehungsweise vergebene Belobigungen laut Ergebnisdaten."
        }
        "fairplayViolations" => {
            "Gespeicherte Fairplay-Verstoßdaten. Codes werden hier nicht bewertet oder übersetzt."
        }
        _ => "",
    };
    if !text.is_empty() {
        return text.into();
    }
    if let Some(base) = key.strip_suffix("Factor100") {
        return format!(
            "Interner Faktor für {base}, in Hundertsteln gespeichert: 150 entspricht Faktor 1,50. Welche Bonusstufe angewendet wird, hängt vom zugehörigen Feld ab."
        );
    }
    if let Some(base) = key.strip_suffix("Factor10") {
        return format!(
            "Interner Faktor für {base}, in Zehnteln gespeichert: 20 entspricht Faktor 2,0. Genaue Anwendung noch nicht geprüft."
        );
    }
    let families = [
        ("booster", "Persönliche Reserve / Booster"),
        ("order", "Clan-/Kampfreserve"),
        ("event", "Aufgabe / Ereignis"),
        ("original", "Ausgangswert vor weiteren Berechnungsstufen"),
        ("subtotal", "Zwischensumme"),
        ("factual", "Berechneter Zwischenwert"),
        ("premium", "Premiumbezogener Wert"),
        ("appliedPremium", "Angewendeter Premiumwert"),
        ("premSquad", "Premium-/Zugbonus"),
        ("squad", "Zugbezogener Wert"),
        ("wotPlus", "WoT-Plus-Wert"),
        ("referral", "Empfehlungsprogramm-Wert"),
        ("achievement", "Auszeichnungsbezogener Wert"),
        ("bp", "Battle-Pass-bezogener Wert"),
        ("destructibles", "Zerstörbare Umgebungsobjekte"),
        ("avatar", "Spielerbezogener Zusatzwert"),
        ("commonNum", "Gefechtsweiter Moduszähler"),
    ];
    for (prefix, meaning) in families {
        if key.starts_with(prefix) {
            return format!(
                "{meaning}: technisches Feld {key}. Einheit, Unterstruktur und genaue Abgrenzung sind noch nicht vollständig geprüft. Der Originalwert wird unverändert angezeigt; nicht als zusätzlichen Gewinn addieren."
            );
        }
    }
    "Noch nicht zugeordnetes technisches Feld. Der Originalwert bleibt sichtbar; Bedeutung, Einheit und mögliche Codes sind noch nicht geprüft.".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_blocks_and_unknown_values_remain_visible() {
        let mut bytes = 0x1134_3212u32.to_le_bytes().to_vec();
        bytes.extend_from_slice(&3u32.to_le_bytes());
        for block in [
            br#"{"unknown":{"null":null,"zero":0}}"#.as_slice(),
            br#"[{}, {"future":[1,true]}]"#,
            br#"{"third":42}"#,
        ] {
            bytes.extend_from_slice(&(block.len() as u32).to_le_bytes());
            bytes.extend_from_slice(block);
        }
        bytes.extend_from_slice(&[0, 255, 17]);
        let document = read(&bytes, "test".into()).unwrap();
        assert_eq!(document.blocks.len(), 3);
        assert!(document.blocks[0]["unknown"]["null"].is_null());
        assert_eq!(document.blocks[0]["unknown"]["zero"], 0);
        assert_eq!(document.stream, [0, 255, 17]);
        assert!(explanation("unknown_new_field").contains("nicht geprüft"));
    }
}
