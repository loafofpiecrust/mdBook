
use handlebars::{Handlebars, RenderError, RenderContext, Helper, Renderable};

pub fn camel_case(h: &Helper, _r: &Handlebars, rc: &mut RenderContext) -> Result<(), RenderError> {
    let input = h.param(0).unwrap().value().as_str().unwrap();
    let mut res = String::with_capacity(input.len());
    let mut cap = false;
    for c in input.chars() {
        if c == '-' {
            cap = true;
        } else {
            if cap {
                for c in c.to_uppercase() {
                    res.push(c);
                }
                cap = false;
            } else {
                res.push(c);
            }
        }
    }
    write!(rc.writer, "{}", res)?;
    Ok(())
}

pub fn jsonify(h: &Helper, _r: &Handlebars, rc: &mut RenderContext) -> Result<(), RenderError> {
    let input = h.param(0).unwrap().value();
    println!("jsonified! {}", input);
    write!(rc.writer, "{}", input)?;
    Ok(())
}
