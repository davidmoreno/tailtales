mod record;

fn main() {
    // get a filename from the command line
    let filename = std::env::args().nth(1).expect("need a filename");
    println!("reading from file: {}", filename);

    let mut record_list = record::RecordList::new();
    record_list.readfile(&filename);

    println!("File has {} lines", record_list.records.len());

    println!("First line is: {:?}", record_list.records[0]);
}
