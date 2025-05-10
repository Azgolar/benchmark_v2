use std::{env, fs, path::Path, process::Command};
use core_affinity;
use getopts::Options;

#[derive(Debug)]
struct Settings 
{
    programm: String,   // Name des auszuführenden Programms
    kerne: Vec<i32>,    // Kerne für das Pinning
    n: Vec<i32>,        // Eingabegrößen für Benchmarking
    t: i32,             // Anzahl der Threads für Benchmarking
    log: String,         // Name der Logdatei
    flagge: bool         // Ausgabe der Einstellungen
}

#[derive(Debug)]
struct ProzessorSpecs 
{
   pub name: String,       // Name des Prozessors
   pub logisch: i32,       // Anzahl der logischen Kerne
   pub physisch: i32,      // Anzahl der physischen Kerne
   pub threads: i32       // Anzahl der Threads
}

fn starten(einstellungen: &Settings) 
{
    // Argumente formatieren
    let formatiert = |v: &Vec<i32>| 
    {
        v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")
    };

    let kerne_formatiert = formatiert(&einstellungen.kerne);
    let n_formatiert     = formatiert(&einstellungen.n);

    // Programm starten und Ausgabe einlesen
    let output = Command::new(format!("./{}", einstellungen.programm))
        .arg(format!("[{}]", kerne_formatiert))
        .arg(format!("[{}]", n_formatiert))
        .arg(einstellungen.t.to_string())
        .output();

    match output 
    {
        Ok(out) => 
        {
            if out.status.success() 
            {
                println!("\nBenchmarking erfolgreich beendet\n");

                // Benchmarking Programm gibt Laufzeit als String zurück
                let rückgabe = String::from_utf8_lossy(&out.stdout).trim().to_string();

                // umwandeln
                let laufzeit: Vec<i32> = rückgabe.split(',').filter_map(|s| s.trim()
                    .parse::<i32>().ok()).collect();
                
                // Debug
                println!("Rückgabe-Vektor: {:?}", laufzeit);
            } 
            else 
            {
                println!("\nBenchmarking wurde mit Fehler beendet: {}\n", out.status);
                std::process::exit(1);
            }
        }
        Err(fehler) => 
        {
            println!("\nFehler beim Starten des Benchmarking-Programms: {}\n", fehler);
            std::process::exit(1);
        }
    }
}


fn main() 
{
    let prozessor: ProzessorSpecs = ProzessorSpecs::new();
    let einstellungen: Settings = Settings::new();

    // Debug
    println!("{:#?}", einstellungen);
    println!("{:#?}", prozessor);

    // benchmarking starten
    starten(&einstellungen);


}

/*
    Parsen der übergebenen Paremeter 
*/
impl Settings 
{
    pub fn new() -> Self 
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
            "-b".into(), "15-19".into(),         
            "-c".into(), "[1,2,3]".into(),
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
            Settings::fehlerausgabe("Parameter a nicht gefunden"));
        if !Path::new(&programm).is_file() 
        {
             Settings::fehlerausgabe("das auszuführende Programm existiert nicht");
        }

        // Parameter b parsen
        let b: String = gefunden.opt_str("b").unwrap_or_else(|| 
            Settings::fehlerausgabe("Parameter b wurde nicht gefunden"));
        let kerne: Vec<i32> = Settings::kern_umwandeln(&b).unwrap_or_else(|_| 
            Settings::fehlerausgabe("Parameter b hat falsches Format"));

        // Parameter c parsen
        let c: String = gefunden.opt_str("c").unwrap_or_else(|| 
            Settings::fehlerausgabe("Parameter c wurde nicht gefunden"));
        let n: Vec<i32> = Settings::n_umwandeln(&c).unwrap_or_else(|_| 
            Settings::fehlerausgabe("Parameter c hat falsches Format"));

        // Parameter d parsen
        let d: String = gefunden.opt_str("d").unwrap_or_else(|| 
            Settings::fehlerausgabe("Parameter d nicht gefunden"));
        let t: i32 = d.parse::<i32>().unwrap_or_else(|_| 
            Settings::fehlerausgabe("Parameter d hat falsches Format"));

        // Parameter e parsen
        let log: String = gefunden.opt_str("e").unwrap_or_else(|| 
            Settings::fehlerausgabe("Parameter e nicht gefunden"));

        // Parameter f parsen
        let flagge: bool = gefunden.opt_present("f");

        Settings { programm, kerne, n, t, log, flagge}
    }

    // Hilfsfunktion für Fehlerausgabe
    fn fehlerausgabe(fehler: &str) -> ! 
    {
        println!("\n{}. Benutzung siehe -h\n", fehler);
        std::process::exit(1);
    }

    /*
        Wandelt einen String mit Zahlen in einen Vektor aus integer um  
    */
    fn n_umwandeln(umwandeln: &str) -> Result<Vec<i32>, ()> 
    {
        let mut zahlen: Vec<i32> = Vec::new();

        // Format: [1,2,3]
        if umwandeln.starts_with('[') && umwandeln.ends_with(']') 
        {
            let innen: &str = &umwandeln[1..umwandeln.len() - 1];
            for i in innen.split(',') 
            {
                let num: i32 = i.trim().parse::<i32>().map_err(|_| ())?;
                zahlen.push(num);
            }
            zahlen.sort();
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
    fn kern_umwandeln(umwandeln: &str) -> Result<Vec<i32>, ()> 
    {
        let mut zahlen: Vec<i32> = Vec::new();

        // Format: [1,2,3]
        if umwandeln.starts_with('[') && umwandeln.ends_with(']') 
        {
            let innen: &str = &umwandeln[1..umwandeln.len() - 1];
            for i in innen.split(',') 
            {
                let num: i32 = i.trim().parse::<i32>().map_err(|_| ())?;
                zahlen.push(num);
            }
            zahlen.sort()
        }
        else if umwandeln.contains("-")
        {
            // Format: "a-b"
            let parts: Vec<&str> = umwandeln.split('-').collect();
            
            if parts.len() != 2 
            {
                return Err(());
            }

            let a = parts[0].trim().parse::<i32>().map_err(|_| ())?;
            let b = parts[1].trim().parse::<i32>().map_err(|_| ())?;

            if a > b
            {
                return Err(());
            }
        
            for i in a..b 
            {
                zahlen.push(i);
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
        let logisch: i32 = core_affinity::get_core_ids().map_or(0, |ids| ids.len() as i32);

        // Anzahl physischer Kerne 
        let physisch = cpuinfo.lines().find(|l| l.starts_with("cpu cores"))
            .and_then(|l| l.splitn(2, ':').nth(1))
            .and_then(|v| v.trim().parse::<i32>().ok())
            .unwrap_or(0);

        // 5) Anzahl Threads pro Kern 
        let threads: i32;
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
            println!("\nFehler beim lesen der Prozessorspezifikationen\n");
            std::process::exit(1);
        } 
        
        ProzessorSpecs { name, logisch, physisch, threads }
    }
}