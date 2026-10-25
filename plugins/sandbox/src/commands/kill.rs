use ferrosa_wit::{Player, server};

pub fn nearest(p: &Player) {
    let camera = p.camera();
    let nearest = server::humans().into_iter().filter(|h| h.vitality() > 0).min_by(|a, b| (a.pos() - camera).length().total_cmp(&(b.pos() - camera).length()));

    match nearest {
        Some(h) => h.set_vitality(0),
        None => p.message("No living human to kill"),
    }
}
