extern crate google_hashcode18_prelim as root;

use root::util::{FileReader, FileWriter, FileIOError};
use root::scheduler::JobScheduler;

#[cfg(test)]

#[test]
fn example_data_set() {
	let mut input = FileReader::new(".\\data\\a_example.in").expect("Couldn't open input file");
	let mut output = FileWriter::new(".\\data\\test_a_example.o").expect("Couldn't open output file");
	let mut runner = JobScheduler::new(input);

	runner.run();
	assert_eq!(runner.output_as_str(),
	           "1 0\n2 1 2\n");
}