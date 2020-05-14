use super::random;

pub struct And {
    pub items: Vec<Box<dyn ToString>>,
    pub sep: String,
}

impl ToString for And {
    fn to_string(&self) -> String {
        let mut res = String::new();
        for (idx, item) in self.items.iter().enumerate() {
            if idx > 0 {
                res += &self.sep;
            }
            res += &item.to_string();
        }
        res
    }
}

pub struct Or {
    pub items: Vec<Box<dyn ToString>>,
}

impl ToString for Or {
    fn to_string(&self) -> String {
        if self.items.len() == 0 {
            return String::from("");
        }
        let rand_idx = random::Rand::rand_int(0, self.items.len());
        let chosen_item = &self.items[rand_idx];
        chosen_item.to_string()
    }
}
