use crossbeam_channel;
use std::{thread, time};

fn parallel_map<T, U, F>(mut input_vec: Vec<T>, num_threads: usize, f: F) -> Vec<U>
where
    F: FnOnce(T) -> U + Send + Copy + 'static,
    T: Send + 'static,
    U: Send + 'static + Default,
{
    // TODO: implement parallel map!
    let input_num = input_vec.len();
    let mut output_vec: Vec<U> = Vec::with_capacity(input_num);
    output_vec.resize_with(input_num, Default::default);

    let mut threads = Vec::new();
    let (input_sender, input_receiver) = crossbeam_channel::unbounded(); // main thread --(input index) --> work thread
    let (output_sender, output_receiver) = crossbeam_channel::unbounded(); // work thread --(output data) --> main thread
    for _ in 0..num_threads {
        let input_receiver = input_receiver.clone();
        let output_sender = output_sender.clone();
        threads.push(thread::spawn(move || {
            while let Ok((input_index, input_val)) = input_receiver.recv() {
                output_sender.send((input_index, f(input_val))).expect("Tried writing to channel, but there are no receivers!");
            }
        }));
    }
    for i in 0..input_num {
        input_sender.send((input_num - i - 1, input_vec.pop().unwrap())).expect("Tried writing to channel, but there are no receivers!");
    }
    drop(input_sender);
    
    for thread in threads {
        thread.join().expect("Panic occurred in thread");
    }
    drop(output_sender);
    
    while let Ok((output_index, output_val)) = output_receiver.recv() {
        output_vec[output_index] = output_val;
    }
    
    output_vec
}

fn main() {
    let v = vec![6, 7, 8, 9, 10, 1, 2, 3, 4, 5, 12, 18, 11, 5, 20];
    let squares = parallel_map(v, 10, |num| {
        println!("{} squared is {}", num, num * num);
        thread::sleep(time::Duration::from_millis(500));
        num * num
    });
    println!("squares: {:?}", squares);
}
