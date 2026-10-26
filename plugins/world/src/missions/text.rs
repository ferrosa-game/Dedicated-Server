pub(super) fn mission_string(file: &str, a: &str, b: &str, project: &str, high_value: bool) -> String {
    let mut text = String::new();

    match rand::random_range(0..3) {
        0 => text += "We have received reports that ",
        1 => text += "Reports indicate that ",
        _ => {}
    }

    text += &match rand::random_range(0..3) {
        0 => format!("the laboratory at {a} and {b} "),
        1 => format!("the lab at {a} and {b} "),
        _ => format!("{a} and {b} laboratory "),
    };

    text += match rand::random_range(0..3) {
        0 => "acquired a disk",
        1 => "has disks containing info",
        _ => "may have disks",
    };

    text += &format!(" the filename is {file} ");
    text += &if high_value { format!("project {project} is high value") } else { format!("project {project} is normal value") };

    text
}

pub(super) fn streets_part(a: &str, b: &str) -> String {
    match rand::random_range(0..3) {
        0 => format!("the laboratory at {a} and {b} has the data. "),
        1 => format!("the lab at {a} and {b} has made great strides. "),
        _ => format!("we are increasing development at the {a} and {b} laboratory. "),
    }
}

pub(super) fn project_part(project: &str) -> String {
    match rand::random_range(0..3) {
        0 => format!("project {project} has made progress in many areas. "),
        1 => format!("there are new developments in the {project} project. "),
        _ => format!("the {project} project will revolutionize the industry. "),
    }
}

pub(super) fn worth_part(boosted: bool) -> &'static str {
    if boosted {
        return if rand::random_range(0..2) == 1 { "this is a high priority project. " } else { "this project is extremely high value. " };
    }

    match rand::random_range(0..4) {
        1 => "the company picnic is next saturday. ",
        2 => "next friday is hawaiian shirt day. ",
        3 => "we need to create more synergy. ",
        _ => "we are now adding coversheets to tps reports. ",
    }
}
