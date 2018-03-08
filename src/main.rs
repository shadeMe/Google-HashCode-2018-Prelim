extern crate google_hashcode18_prelim as root;
extern crate itertools;

use root::util::{FileReader, FileWriter};
use root::scheduler::JobScheduler;
use self::itertools::Itertools;

fn main() {
	let input = vec![".\\data\\a_example.in",
                                ".\\data\\b_should_be_easy.in",
                                ".\\data\\c_no_hurry.in",
                                ".\\data\\d_metropolis.in",
                                ".\\data\\e_high_bonus.in"];
	let output = vec![".\\data\\a_example.o",
	                            ".\\data\\b_should_be_easy.o",
				                ".\\data\\c_no_hurry.o",
				                ".\\data\\d_metropolis.o",
				                ".\\data\\e_high_bonus.o"];
	let mut total_score: u64 = 0;

	input.iter().zip(output.iter()).collect_vec().into_iter().foreach(|(i, o)| {
		let input = FileReader::new(i).expect("Couldn't open input file");
		let mut output = FileWriter::new(o).expect("Couldn't open output file");
		let mut runner = JobScheduler::new(input);

		println!("\n\n ============= Input {} ==================\n\n", i);

		runner.run();
		runner.write_output(&mut output);
		total_score += runner.calculate_score();
	});

	println!("\n\nTotal score: {}", total_score);
}