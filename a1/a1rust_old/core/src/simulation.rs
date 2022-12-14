use std::io::Write;

use crate::{
    area::Area,
    cmd::{Cmd, CmdType, ParseError, ParseResult},
};

pub struct Simulator(Vec<Simulation>);

impl Simulator {
    pub fn new(files: Vec<&str>, run_option: RunOption) -> Self {
        let mut simulations = vec![];
        for file in files {
            let simulation = Simulation::new(file, run_option.clone());
            simulations.push(simulation);
        }
        Self(simulations)
    }

    pub fn run(&mut self) {
        for simulation in self.0.iter_mut() {
            simulation.run();
        }
    }
}

#[derive(Debug, Clone)]
pub enum RunOption {
    All,
    FirstFit,
    BestFit,
    WorstFit,
}

impl From<(bool, bool, bool, bool)> for RunOption {
    fn from((all, first, best, worst): (bool, bool, bool, bool)) -> Self {
        if all {
            Self::All
        } else if first {
            Self::FirstFit
        } else if best {
            Self::BestFit
        } else if worst {
            Self::WorstFit
        } else {
            Self::FirstFit
        }
    }
}

pub struct Simulation(Vec<Run>, String);

impl Simulation {
    pub fn new(file: &str, ty: RunOption) -> Self {
        let runs = match ty {
            RunOption::All => {
                let first_fit_run = Run::new(file, AllocationMethod::FirstFit)
                    .expect("Failed to create first-fit run");
                let best_fit_run = Run::new(file, AllocationMethod::BestFit)
                    .expect("Failed to create best-fit run");
                let worst_fit_run = Run::new(file, AllocationMethod::WorstFit)
                    .expect("Failed to create worst-fit run");
                vec![first_fit_run, best_fit_run, worst_fit_run]
            }
            RunOption::FirstFit => {
                let first_fit_run = Run::new(file, AllocationMethod::FirstFit)
                    .expect("Failed to create first-fit run");
                vec![first_fit_run]
            }
            RunOption::BestFit => {
                let best_fit_run = Run::new(file, AllocationMethod::BestFit)
                    .expect("Failed to create best-fit run");
                vec![best_fit_run]
            }
            RunOption::WorstFit => {
                let worst_fit_run = Run::new(file, AllocationMethod::WorstFit)
                    .expect("Failed to create worst-fit run");
                vec![worst_fit_run]
            }
        };
        Simulation(runs, file.to_owned())
    }

    pub fn run(&mut self) {
        let areas = self
            .0
            .iter_mut()
            .map(|r| r.run())
            .collect::<Vec<(Area, AllocationMethod)>>();
        let mut out_string = String::new();
        for (area, method) in areas.iter() {
            let method_str = method.to_string();
            let used_blocks = area
                .used_blocks
                .iter()
                .map(|b| b.to_string())
                .collect::<Vec<String>>()
                .join("\n");
            let free_blocks = area
                .free_blocks
                .iter()
                .map(|b| b.to_string())
                .collect::<Vec<String>>()
                .join("\n");
            let fragmentation = area.fragmentation();
            out_string.push_str(&format!(
                "{}\nUsed Blocks:\n{}\nFree Blocks:\n{}\nFragmentation:\n{}\n",
                method_str, used_blocks, free_blocks, fragmentation
            ));
            let file_name = self.1.clone();
            let mut out_file = std::fs::File::create(format!("{}_out.txt", file_name)).unwrap();
            let mut raw_file = std::fs::File::create(format!("{}_raw_out.txt", file_name)).unwrap();
            out_file.write_all(out_string.as_bytes()).unwrap();
            raw_file
                .write_all(format!("{:#?}", area).as_bytes())
                .unwrap();
        }
    }
}

#[derive(Debug, Default, Clone)]
pub enum AllocationMethod {
    #[default]
    FirstFit,
    BestFit,
    WorstFit,
}

impl std::fmt::Display for AllocationMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FirstFit => write!(f, "First Fit"),
            Self::BestFit => write!(f, "Best Fit"),
            Self::WorstFit => write!(f, "Worst Fit"),
        }
    }
}

pub struct Run {
    pub method: AllocationMethod,
    pub area: Area,
    pub cmds: Vec<Cmd>,
}

impl Run {
    pub fn new(file: &str, method: AllocationMethod) -> Result<Self, ParseError> {
        let res = Self::from_file(file);
        match res {
            Ok((area, cmds)) => Ok(Run { method, area, cmds }),
            Err(err) => Err(err),
        }
    }

    pub fn from_file(path: &str) -> ParseResult<(Area, Vec<Cmd>)> {
        let inp = std::fs::read_to_string(path)?;
        Self::from_str(&inp)
    }

    pub fn from_str(inp: &str) -> ParseResult<(Area, Vec<Cmd>)> {
        let itr = inp.split_whitespace().into_iter();
        let size = itr.clone().next();
        match size {
            Some(size) => {
                let size = size.parse()?;
                let area = Area::new(size);
                let mut cmds = Vec::new();
                for cmd in itr.skip(1) {
                    let cmd = Cmd::try_from(cmd)?;
                    cmds.push(cmd);
                }
                Ok((area, cmds))
            }
            None => Err(ParseError::MissingParameters("size".to_owned())),
        }
    }

    pub fn run(&mut self) -> (Area, AllocationMethod) {
        for cmd in self.cmds.iter() {
            match cmd.ty {
                CmdType::Alloc => {
                    println!(
                        "Allocating block {} with {} bytes",
                        cmd.block_id.unwrap(),
                        cmd.size.unwrap()
                    );
                    let block_id = cmd.block_id.unwrap();
                    let size = cmd.size.unwrap();
                    let res = match self.method {
                        AllocationMethod::FirstFit => self.area.alloc_first_fit(block_id, size),
                        AllocationMethod::BestFit => self.area.alloc_best_fit(block_id, size),
                        AllocationMethod::WorstFit => self.area.alloc_worst_fit(block_id, size),
                    };
                    match res {
                        Ok(_) => (),
                        Err(err) => println!("Error: {}", err),
                    }
                }
                CmdType::Dealloc => {
                    println!("Deallocating block {}.", cmd.block_id.unwrap());
                    let block_id = cmd.block_id.unwrap();
                    match self.area.dealloc(block_id) {
                        Ok(_) => (),
                        Err(err) => println!("Error: {}", err),
                    }
                }
                CmdType::Compact => match self.area.compact() {
                    Ok(_) => (),
                    Err(err) => println!("Error: {}", err),
                },
            }
        }
        println!("Fragmentation: {}.", self.area.fragmentation());
        (self.area.clone(), self.method.clone())
    }
}

mod run_tests {
    use crate::cmd::CmdType;

    #[test]
    fn test_from_file() {
        let run = super::Run::new("../input/test.txt", super::AllocationMethod::FirstFit).unwrap();
        assert_eq!(run.area.size, 1000);
        assert_eq!(run.cmds.len(), 6);

        let cmd1 = &run.cmds[0];
        assert_eq!(cmd1.block_id, Some(0));
        assert_eq!(cmd1.size, Some(100));
        assert_eq!(cmd1.ty, CmdType::Alloc);

        let cmd2 = &run.cmds[1];
        assert_eq!(cmd2.block_id, Some(1));
        assert_eq!(cmd2.size, Some(100));
        assert_eq!(cmd2.ty, CmdType::Alloc);

        let cmd3 = &run.cmds[2];
        assert_eq!(cmd3.block_id, Some(2));
        assert_eq!(cmd3.size, Some(500));
        assert_eq!(cmd3.ty, CmdType::Alloc);

        let cmd4 = &run.cmds[3];
        assert_eq!(cmd4.block_id, Some(1));
        assert_eq!(cmd4.ty, CmdType::Dealloc);

        let cmd5 = &run.cmds[4];
        assert_eq!(cmd5.block_id, Some(3));
        assert_eq!(cmd5.size, Some(200));
        assert_eq!(cmd5.ty, CmdType::Alloc);

        let cmd6 = &run.cmds[5];
        assert_eq!(cmd6.block_id, Some(2));
        assert_eq!(cmd6.ty, CmdType::Dealloc);
    }

    #[test]
    fn test_from_str() {
        let input = r#"1000
A;0;100
A;1;100
A;2;500
D;1
A;3;200
D;2
        "#;

        let (area, cmds) = super::Run::from_str(input).unwrap();
        let run = super::Run {
            method: super::AllocationMethod::FirstFit,
            area,
            cmds,
        };
        assert_eq!(run.area.size, 1000);
        assert_eq!(run.cmds.len(), 6);

        let cmd1 = &run.cmds[0];
        assert_eq!(cmd1.block_id, Some(0));
        assert_eq!(cmd1.size, Some(100));
        assert_eq!(cmd1.ty, CmdType::Alloc);

        let cmd2 = &run.cmds[1];
        assert_eq!(cmd2.block_id, Some(1));
        assert_eq!(cmd2.size, Some(100));
        assert_eq!(cmd2.ty, CmdType::Alloc);

        let cmd3 = &run.cmds[2];
        assert_eq!(cmd3.block_id, Some(2));
        assert_eq!(cmd3.size, Some(500));
        assert_eq!(cmd3.ty, CmdType::Alloc);

        let cmd4 = &run.cmds[3];
        assert_eq!(cmd4.block_id, Some(1));
        assert_eq!(cmd4.ty, CmdType::Dealloc);

        let cmd5 = &run.cmds[4];
        assert_eq!(cmd5.block_id, Some(3));
        assert_eq!(cmd5.size, Some(200));
        assert_eq!(cmd5.ty, CmdType::Alloc);

        let cmd6 = &run.cmds[5];
        assert_eq!(cmd6.block_id, Some(2));
        assert_eq!(cmd6.ty, CmdType::Dealloc);
    }
}
