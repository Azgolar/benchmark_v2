use std::{fs, env, path::Path};
use core_affinity;
use getopts::Options;

#[derive(Debug)]
struct Einstellungen 
{
    programm: String,   // Name des auszuführenden Programms
    kerne: Vec<i32>,    // Kerne für das Pinning
    n: Vec<i32>,        // Eingabegrößen für Benchmarking
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

fn main() 
{
    let prozessor: ProzessorSpecs = ProzessorSpecs::new();
    let einstellungen: Einstellungen = Einstellungen::new();

    println!("{:#?}", einstellungen);

    println!("{:#?}", prozessor);


}

/*
    Parsen der übergebenen Paremeter 
*/
impl Einstellungen 
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
        // optinale Parameter
        parameter.optflag("e", "", "");
        parameter.optflag("h", "", "");

        // Test-Einstellungen
        let test_args: Vec<String> = vec![
            "-a".into(), "kette.txt".into(),        
            "-b".into(), "15-19".into(),            
            "-c".into(), "[1,2,3]".into(),
            "-d".into(), "log".into(),
            "-e".into(),
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
            println!("-d <Name der Logdatei zum Speichern der Ergebnisse>");
            println!("\noptional:");
            println!("-e <Ausgabe der CPU Spezifikationen");
            std::process::exit(0);
        }

        // Parameter a parsen
        let programm: String = gefunden.opt_str("a").unwrap_or_else(|| 
            { Einstellungen::fehlerausgabe("Parameter a nicht gefunden")});
        if !Path::new(&programm).is_file() 
        {
             Einstellungen::fehlerausgabe("das auszuführende Programm existiert nicht");
        }

        // Parameter b parsen
        let b: String = gefunden.opt_str("b").unwrap_or_else(|| 
            { Einstellungen::fehlerausgabe("Parameter b wurde nicht gefunden")});
        let kerne: Vec<i32> = Einstellungen::kern_umwandeln(&b).unwrap_or_else(|_| 
            { Einstellungen::fehlerausgabe("Parameter b hat falsches Format")});

        // Parameter c parsen
        let c: String = gefunden.opt_str("c").unwrap_or_else(|| 
            { Einstellungen::fehlerausgabe("Parameter c wurde nicht gefunden")});
        let n: Vec<i32> = Einstellungen::n_umwandeln(&c).unwrap_or_else(|_| 
            { Einstellungen::fehlerausgabe("Parameter c hat falsches Format")});

        // Parameter d parsen
        let log: String = gefunden.opt_str("d").unwrap_or_else(|| 
            { Einstellungen::fehlerausgabe("Parameter d nicht gefunden")});

        // Parameter e parsen
        let flagge: bool = gefunden.opt_present("e");

        Einstellungen { programm, kerne, n, log, flagge}
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