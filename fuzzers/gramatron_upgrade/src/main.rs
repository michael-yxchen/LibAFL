use std::{
    fs,
    io::{BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
};

use libafl::generators::gramatron::Automaton;

fn read_automaton_from_postcard<P: AsRef<Path>>(path: P) -> Automaton {
    let file = fs::File::open(path).unwrap();
    let mut reader = BufReader::new(file);
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer).unwrap();
    postcard_0_7::from_bytes(&buffer).unwrap()
}

fn write_automaton_to_postcard<P: AsRef<Path>>(path: P, automaton: &Automaton) {
    let mut file = fs::File::create(path).unwrap();
    let vec = postcard_1_0::to_allocvec(automaton).unwrap();
    file.write_all(&vec).unwrap()
}

fn write_automaton_to_json<P: AsRef<Path>>(path: P, automaton: &Automaton) {
    let file = fs::File::create(path).unwrap();
    let writer = BufWriter::new(file);
    serde_json::to_writer(writer, automaton).unwrap()
}

fn main() {
    let old_postcard_path = PathBuf::from("../baby_fuzzer_gramatron/auto.postcard");
    let new_postcard_path = PathBuf::from("auto.postcard");
    let json_path = PathBuf::from("auto.json");

    let automaton = read_automaton_from_postcard(old_postcard_path);
    write_automaton_to_postcard(new_postcard_path, &automaton);
    write_automaton_to_json(json_path, &automaton);
}
