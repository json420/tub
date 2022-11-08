use bathtub_db::commands::{get_args, run};


fn main()   
{   
    run(&mut get_args());
}
