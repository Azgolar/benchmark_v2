use std::{env, fs, path::Path, process::Command, fs::OpenOptions, io::Write};
use core_affinity;
use getopts::Options;
use core_affinity::{get_core_ids, set_for_current};

#[derive(Debug)]
struct Settings 
{
    programm: String,   // Name des auszuführenden Programms
    kerne: Vec<u32>,    // Kerne für das Pinning
    n: Vec<u32>,        // Eingabegrößen für Benchmarking
    t: u32,             // Anzahl der Threads für Benchmarking
    log: String,         // Name der Logdatei
    flagge: bool         // Ausgabe der Einstellungen
}

#[derive(Debug)]
struct ProzessorSpecs 
{
   name: String,       // Name des Prozessors
   logisch: u32,       // Anzahl der logischen Kerne
   physisch: u32,      // Anzahl der physischen Kerne
   threads: u32       // Anzahl der Threads
}

/*
    führt das Benchmarking durch
*/
fn starten(einstellungen: &Settings) -> Vec<f64> 
{
    // 1. Kerne und n für Befehl formatieren
    let k: String = einstellungen.kerne.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",");
    let n: String = einstellungen.n.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",");

    // 2. Argumente bauen
    let args: Vec<String> = vec![format!("[{}]", k), format!("[{}]", n), einstellungen.t.to_string()];

    // 3. Nur die Shell-Zeile ausgeben
    if einstellungen.flagge 
    {
        println!("Benchmarking ausgeführt mit: ./{} {}\n", einstellungen.programm, args.join(" "));
    }

    // Benchmarking ausführen
    let ausführen = Command::new(format!("./{}", einstellungen.programm))
        .args(&args)
        .output()
        .unwrap_or_else(|e| 
            {
                fehlerausgabe(&format!("Fehler beim Starten des Benchmarking-Programms: {}", e))
            });

    // 5. Exit-Status prüfen
    if !ausführen.status.success() 
    {
        fehlerausgabe(&format!("Benchmarking wurde mit Fehler beendet: {}", ausführen.status));
    }

    println!("\nBenchmarking erfolgreich beendet\n");

    // Laufzeit parsen
    let rückgabe = String::from_utf8_lossy(&ausführen.stdout).trim().to_string();
    let laufzeit: Vec<f64> = rückgabe.split(',').filter_map(|s| s.trim().parse::<f64>().ok()).collect();

    println!("Rückgabe-Vektor: {:?}", laufzeit);
    laufzeit
}


fn fehlerausgabe(fehler: &str) -> !
{
    println!("\n{}\n", fehler);
    std::process::exit(1);
}

fn speichern(einstellungen: &Settings, prozessor: &ProzessorSpecs, laufzeit: &[f64]) 
{
    // öffnen der Logdatei und überschreiben falls vorhanden
    let mut file = OpenOptions::new().write(true).create(true).truncate(true)
        .open(&einstellungen.log)
        .unwrap_or_else(|fehler| 
            {
                fehlerausgabe(&format!("Fehler beim Öffnen der Logdatei {}", fehler))
            });

    // Prozessorinformationen in erster Zeile speichern
    writeln!(file, "{}{}{}{}", prozessor.name, prozessor.physisch, prozessor.logisch, prozessor.threads)
        .unwrap_or_else(|_| 
            {
                fehlerausgabe(&format!("Fehler beim schreiben der Prozessorinformationen"))
            });

        
    // Laufzeiten speichern
    for (&n, &zeit) in einstellungen.n.iter().zip(laufzeit.iter()) 
    {
        writeln!(file, "{},{}", n, zeit).unwrap_or_else(|_| 
            {
                fehlerausgabe(&format!("Fehler beim schreiben der Laufzeiten"))
            });
    }

    println!("Ergebnisse in '{}' geschrieben.", einstellungen.log);
}

/*
    Ausgeben der Einstellungen
*/
fn ausgeben(einstellungen: &Settings, prozessor: &ProzessorSpecs)
{
    // Debug
    println!("{:#?}", einstellungen);
    println!("{:#?}", prozessor);
}

/*
    Pinnt das Programm
    1. freien physischen Kern mit höchster id suchen und davon die niedrigsten logischen Kern
    2. falls kein freier physischer Kern, höchsten freien logischen Kern nehmen
    3. falls immer noch keiner frei, letzten logischen Kern als Fallback
*/
fn pinnen(einstellungen: &Settings, prozessor: &ProzessorSpecs) 
{
    let mut frei: i32 = -1;

    // 1. freien physischen Kern mit höchster id suchen und davon die niedrigsten logischen Kern
    for phys in (0..prozessor.physisch).rev() 
    {
        let start = phys * prozessor.threads;
        let end = start + prozessor.threads;
        if (start..end).all(|id| !einstellungen.kerne.contains(&id)) 
        {
            frei = start as i32;
            break;
        }
    }

    // 2. falls kein freier physischer Kern, höchsten freien logischen Kern nehmen
    if frei == -1 
    {
        for logik in (0..prozessor.logisch).rev() 
        {
            if !einstellungen.kerne.contains(&logik) 
            {
                frei = logik as i32;
                break;
            }
        }
    }

    // 3. falls immer noch keiner frei, letzten logischen Kern als Fallback
    if frei == -1 
    {
        frei = (prozessor.logisch - 1) as i32;
    }

    // pinnen
    let liste = get_core_ids().unwrap();
    let id = liste.get(frei as usize).unwrap_or_else(|| 
        fehlerausgabe("Kann Programm nicht auf Kern pinnen"));
    set_for_current(*id);

    if einstellungen.flagge
    {
        println!("\nPinne Programm zu loggen auf logischen Kern {}\n", frei);
    }
}

fn main() 
{
    let prozessor: ProzessorSpecs = ProzessorSpecs::new();
    let einstellungen: Settings = Settings::new(&prozessor);

    // Pinnen des Programms um das Benchmarking nicht zu stören
    pinnen(&einstellungen, &prozessor);

    // benchmarking starten
    let laufzeit: Vec<f64> = starten(&einstellungen);

    // speichern
    speichern(&einstellungen, &prozessor, &laufzeit);

    // Einstellungen ausgeben
    if einstellungen.flagge
    {
        ausgeben(&einstellungen, &prozessor);
    }
}

/*
    Parsen der übergebenen Paremeter 
*/
impl Settings 
{
    pub fn new(prozessor: &ProzessorSpecs) -> Self 
    {
        // getopt Einstellungen
        let mut parameter = Options::new();
        // Pflichtparameter
        parameter.optopt("a", "","", "");
        parameter.optopt("b", "", "", "");
        parameter.optopt("c", "", "", "");
        parameter.optopt("d", "", "", "");
        parameter.optopt("e", "", "", "");
        // optinale Parameter
        parameter.optflag("f", "", "");
        parameter.optflag("h", "", "");

        // Test-Einstellungen
        let test_args: Vec<String> = vec![
            "-a".into(), "kette.txt".into(),        
            "-b".into(), "12-18".into(),         
            "-c".into(), "[4,5,6]".into(),
            "-d".into(), "4".into(),   
            "-e".into(), "log".into(),
            "-f".into(),
            ];
        let gefunden = parameter.parse(&test_args).unwrap();

        //let gefunden = parameter.parse(&env::args()
          //  .skip(1).collect::<Vec<_>>()).unwrap_or_else(|e| 
           // { Einstellungen::fehlerausgabe(&format!("Fehler beim Parsen des Arguments: {}", e))});

        // Hilfe ausgeben
        if gefunden.opt_present("h") 
        {
            println!("\nPflichtparameter:");
            println!("-a <Name des auszuführenden Programms>");
            println!("-b <Kern ids für das Pinning: Format [1,7,3,5] oder 3-7>");
            println!("-c <Eingabegrößen n für das auszuführende Programm. Format: [10,80,30,100]>");
            println!("-d <Anzahl Threads für das Benchmarking>");
            println!("-e <Name der Logdatei zum Speichern der Ergebnisse>");
            println!("\noptional:");
            println!("-f <Ausgabe der CPU Spezifikationen");
            std::process::exit(0);
        }

        // Parameter a parsen
        let programm: String = gefunden.opt_str("a").unwrap_or_else(|| 
            fehlerausgabe("Parameter a nicht gefunden. Benutzung siehe -h"));
        if !Path::new(&programm).is_file() 
        {
            fehlerausgabe("Das auszuführende Programm existiert nicht. Benutzung siehe -h");
        }

        // Parameter b parsen
        let b: String = gefunden.opt_str("b").unwrap_or_else(|| 
            fehlerausgabe("Parameter b wurde nicht gefunden. Benutzung siehe -h"));
        let kerne: Vec<u32> = Settings::kern_umwandeln(&b, &prozessor).unwrap_or_else(|_| 
            fehlerausgabe("Parameter b hat falsches Format. Benutzung siehe -h"));

        // Parameter c parsen
        let c: String = gefunden.opt_str("c").unwrap_or_else(|| 
            fehlerausgabe("Parameter c wurde nicht gefunden. Benutzung siehe -h"));
        let n: Vec<u32> = Settings::n_umwandeln(&c).unwrap_or_else(|_| 
            fehlerausgabe("Parameter c hat falsches Format. Benutzung siehe -h"));

        // Parameter d parsen
        let d: String = gefunden.opt_str("d").unwrap_or_else(|| 
            fehlerausgabe("Parameter d nicht gefunden. Benutzung siehe -h"));
        let t: u32 = d.parse::<u32>().unwrap_or_else(|_| 
            fehlerausgabe("Parameter d hat falsches Format. Benutzung siehe -h"));

        // Parameter e parsen
        let log: String = gefunden.opt_str("e").unwrap_or_else(|| 
            fehlerausgabe("Parameter e nicht gefunden. Benutzung siehe -h"));

        // Parameter f parsen
        let flagge: bool = gefunden.opt_present("f");

        Settings { programm, kerne, n, t, log, flagge}
    }

    /*
        Wandelt einen String mit Zahlen in einen Vektor aus integer um  
    */
    fn n_umwandeln(umwandeln: &str) -> Result<Vec<u32>, ()> 
    {
        let mut zahlen: Vec<u32> = Vec::new();

        // Format: [1,2,3]
        if umwandeln.starts_with('[') && umwandeln.ends_with(']') 
        {
            let innen: &str = &umwandeln[1..umwandeln.len() - 1];
            for i in innen.split(',') 
            {
                let num: u32 = i.trim().parse::<u32>().map_err(|_| ())?;
                zahlen.push(num);
            }
            zahlen.sort();
            // mehrfache Zahlen entfernen
            zahlen.dedup();
            Ok(zahlen)
        }
        else 
        {
            return Err(());    
        }
    }
    
    /*
        Wandelt einen String mit Kern ids in einen Vektor aus integer um  
    */
    fn kern_umwandeln(umwandeln: &str, prozessor: &ProzessorSpecs) -> Result<Vec<u32>, ()> 
    {
        let mut zahlen: Vec<u32> = Vec::new();

        // Format: [1,2,3]
        if umwandeln.starts_with('[') && umwandeln.ends_with(']') 
        {
            let innen: &str = &umwandeln[1..umwandeln.len() - 1];
            for i in innen.split(',') 
            {
                let nummer: u32 = i.trim().parse::<u32>().map_err(|_| ())?;
                if nummer < prozessor.logisch
                {
                    zahlen.push(nummer);
                }
                else 
                {
                    return Err(());  
                }

            }
            zahlen.sort();
            // mehrfache Zahlen entfernen
            zahlen.dedup(); 
        }
        else if umwandeln.contains("-")
        {
            // Format: "a-b"
            let parts: Vec<&str> = umwandeln.split('-').collect();
            
            if parts.len() != 2 
            {
                return Err(());
            }

            let a: u32 = parts[0].trim().parse::<u32>().map_err(|_| ())?;
            let b: u32 = parts[1].trim().parse::<u32>().map_err(|_| ())?;

            if a < prozessor.logisch && b < prozessor.logisch && b >= a
            {
                for i in a..=b 
                {
                    zahlen.push(i);
                }         
            } 
            else
            {
                return Err(());
            }
        }
        else 
        {
            return Err(());    
        }

        Ok(zahlen)
    }
}


/*
     sammelt die Prozessorinformationen:
     - Modellname
     - Anzahl logische Kerne
     - Anzahl physische Kerne
     - Anzahl Threads pro Kern
*/
impl ProzessorSpecs 
{
    pub fn new() -> Self 
    {
        let cpuinfo: String = fs::read_to_string("/proc/cpuinfo").unwrap(); 

        // Modellname auslesen
        let name: String = cpuinfo.lines().find(|l| l.starts_with("model name"))
            .and_then(|l| l.splitn(2, ':').nth(1))
            .map(str::trim)
            .unwrap_or("")
            .to_string();

        // Anzahl logischer Kerne 
        let logisch: u32 = core_affinity::get_core_ids().map_or(0, |ids| ids.len() as u32);

        // Anzahl physischer Kerne 
        let physisch = cpuinfo.lines().find(|l| l.starts_with("cpu cores"))
            .and_then(|l| l.splitn(2, ':').nth(1))
            .and_then(|v| v.trim().parse::<u32>().ok())
            .unwrap_or(0);

        // 5) Anzahl Threads pro Kern 
        let threads: u32;
        if physisch > 0 
        {
           threads = logisch / physisch
        } 
        else 
        {
            threads = 0;
        };

        if name == "" || logisch == 0 || physisch == 0 || threads == 0
        {
            fehlerausgabe("Fehler beim lesen der Prozessorspezifikationen");
        } 
        
        ProzessorSpecs { name, logisch, physisch, threads }
    }
}