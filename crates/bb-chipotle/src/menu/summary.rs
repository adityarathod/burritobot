use serde::Serialize;
use thiserror::Error;

use super::Response;

#[derive(Serialize, Debug)]
pub struct Summary {
    pub restaurant_id: i32,
    pub veggie_bowl_price: Price,
    pub chicken_bowl_price: Price,
    pub steak_bowl_price: Price,
}

#[derive(Default)]
pub struct SummaryBuilder {
    restaurant_id: Option<i32>,
    veggie_bowl_price: Option<Price>,
    chicken_bowl_price: Option<Price>,
    steak_bowl_price: Option<Price>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BuildError {
    #[error("missing required fields: {0:?}")]
    MissingFields(Vec<&'static str>),
}

#[derive(Serialize, Debug)]
pub struct Price {
    pub normal_price: f32,
    pub delivery_price: f32,
}

impl PartialEq for Price {
    fn eq(&self, other: &Self) -> bool {
        let normal_price_within_epsilon =
            (self.normal_price - other.normal_price).abs() < f32::EPSILON;
        let delivery_price_within_epsilon =
            (self.delivery_price - other.delivery_price).abs() < f32::EPSILON;

        normal_price_within_epsilon && delivery_price_within_epsilon
    }
}

impl Eq for Price {}

impl Summary {
    pub fn builder() -> SummaryBuilder {
        SummaryBuilder::default()
    }
}

impl SummaryBuilder {
    pub fn restaurant_id(mut self, restaurant_id: i32) -> Self {
        self.restaurant_id = Some(restaurant_id);
        self
    }

    pub fn veggie_bowl_price(mut self, veggie_bowl_price: Price) -> Self {
        self.veggie_bowl_price = Some(veggie_bowl_price);
        self
    }

    pub fn chicken_bowl_price(mut self, chicken_bowl_price: Price) -> Self {
        self.chicken_bowl_price = Some(chicken_bowl_price);
        self
    }

    pub fn steak_bowl_price(mut self, steak_bowl_price: Price) -> Self {
        self.steak_bowl_price = Some(steak_bowl_price);
        self
    }

    pub fn is_complete(&self) -> bool {
        self.restaurant_id.is_some()
            && self.veggie_bowl_price.is_some()
            && self.chicken_bowl_price.is_some()
            && self.steak_bowl_price.is_some()
    }

    pub fn build(self) -> Result<Summary, BuildError> {
        if !self.is_complete() {
            let mut missing_fields = Vec::new();
            if self.restaurant_id.is_none() {
                missing_fields.push("restaurant_id");
            }
            if self.veggie_bowl_price.is_none() {
                missing_fields.push("veggie_bowl_price");
            }
            if self.chicken_bowl_price.is_none() {
                missing_fields.push("chicken_bowl_price");
            }
            if self.steak_bowl_price.is_none() {
                missing_fields.push("steak_bowl_price");
            }
            return Err(BuildError::MissingFields(missing_fields));
        }
        Ok(Summary {
            restaurant_id: self.restaurant_id.unwrap(),
            veggie_bowl_price: self.veggie_bowl_price.unwrap(),
            chicken_bowl_price: self.chicken_bowl_price.unwrap(),
            steak_bowl_price: self.steak_bowl_price.unwrap(),
        })
    }
}

impl TryFrom<Response> for Summary {
    type Error = BuildError;

    fn try_from(res: Response) -> Result<Self, BuildError> {
        // TODO: Implement this
        let mut builder = Summary::builder().restaurant_id(res.restaurant_id);

        for item in res.entrees {
            if builder.is_complete() {
                break;
            }
            if item.item_type.to_lowercase() != "bowl" {
                continue;
            }
            match item.item_name.to_lowercase().replace("bowl", "").trim() {
                "veggie" => {
                    builder = builder.veggie_bowl_price(Price {
                        normal_price: item.unit_price,
                        delivery_price: item.unit_delivery_price,
                    });
                }
                "chicken" => {
                    builder = builder.chicken_bowl_price(Price {
                        normal_price: item.unit_price,
                        delivery_price: item.unit_delivery_price,
                    });
                }
                "steak" => {
                    builder = builder.steak_bowl_price(Price {
                        normal_price: item.unit_price,
                        delivery_price: item.unit_delivery_price,
                    });
                }
                _ => {}
            }
        }
        builder.build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu::get::Item;

    #[test]
    fn summary_builder_is_complete() {
        let builder = Summary::builder()
            .restaurant_id(1)
            .veggie_bowl_price(Price {
                normal_price: 1.0,
                delivery_price: 1.0,
            })
            .chicken_bowl_price(Price {
                normal_price: 1.0,
                delivery_price: 1.0,
            })
            .steak_bowl_price(Price {
                normal_price: 1.0,
                delivery_price: 1.0,
            });
        assert!(builder.is_complete());
    }

    #[test]
    fn summary_builder_is_incomplete() {
        let builder = Summary::builder()
            .restaurant_id(1)
            .veggie_bowl_price(Price {
                normal_price: 1.0,
                delivery_price: 1.0,
            })
            .chicken_bowl_price(Price {
                normal_price: 1.0,
                delivery_price: 1.0,
            });
        assert!(!builder.is_complete());
    }

    #[test]
    fn summary_builder_build() {
        let summary = Summary::builder()
            .restaurant_id(1)
            .veggie_bowl_price(Price {
                normal_price: 1.0,
                delivery_price: 1.0,
            })
            .chicken_bowl_price(Price {
                normal_price: 1.0,
                delivery_price: 1.0,
            })
            .steak_bowl_price(Price {
                normal_price: 1.0,
                delivery_price: 1.0,
            })
            .build();
        assert!(summary.is_ok());
    }

    #[test]
    fn summary_builder_build_missing_fields() {
        let summary = Summary::builder()
            .restaurant_id(1)
            .veggie_bowl_price(Price {
                normal_price: 1.0,
                delivery_price: 1.0,
            })
            .chicken_bowl_price(Price {
                normal_price: 1.0,
                delivery_price: 1.0,
            })
            .build();
        assert!(summary.is_err());

        match summary.unwrap_err() {
            BuildError::MissingFields(fields) => {
                assert_eq!(fields, vec!["steak_bowl_price"]);
            }
        }
    }

    #[test]
    fn summary_from_response() {
        let response = Response {
            restaurant_id: 1,
            entrees: vec![
                Item {
                    item_category: "entree".to_string(),
                    item_type: "Bowl".to_string(),
                    item_id: "1".to_string(),
                    item_name: "Veggie Bowl".to_string(),
                    unit_price: 1.0,
                    unit_delivery_price: 1.0,
                },
                Item {
                    item_category: "entree".to_string(),
                    item_type: "Bowl".to_string(),
                    item_id: "2".to_string(),
                    item_name: "Chicken Bowl".to_string(),
                    unit_price: 2.0,
                    unit_delivery_price: 2.0,
                },
                Item {
                    item_category: "entree".to_string(),
                    item_type: "Bowl".to_string(),
                    item_id: "3".to_string(),
                    item_name: "Steak Bowl".to_string(),
                    unit_price: 3.0,
                    unit_delivery_price: 3.0,
                },
            ],
            sides: vec![],
        };
        let summary = Summary::try_from(response);
        assert!(summary.is_ok());
        let summary = summary.unwrap();
        assert_eq!(summary.restaurant_id, 1);
        assert_eq!(
            summary.veggie_bowl_price,
            Price {
                normal_price: 1.0,
                delivery_price: 1.0
            }
        );
        assert_eq!(
            summary.chicken_bowl_price,
            Price {
                normal_price: 2.0,
                delivery_price: 2.0
            }
        );
        assert_eq!(
            summary.steak_bowl_price,
            Price {
                normal_price: 3.0,
                delivery_price: 3.0
            }
        );
    }

    #[test]
    fn summary_from_incomplete_response() {
        let response = Response {
            restaurant_id: 1,
            entrees: vec![
                Item {
                    item_category: "entree".to_string(),
                    item_type: "Bowl".to_string(),
                    item_id: "1".to_string(),
                    item_name: "Veggie Bowl".to_string(),
                    unit_price: 1.0,
                    unit_delivery_price: 1.0,
                },
                Item {
                    item_category: "entree".to_string(),
                    item_type: "Bowl".to_string(),
                    item_id: "2".to_string(),
                    item_name: "Chicken Bowl".to_string(),
                    unit_price: 2.0,
                    unit_delivery_price: 2.0,
                },
            ],
            sides: vec![],
        };
        let summary = Summary::try_from(response).err().unwrap();
        assert_eq!(summary, BuildError::MissingFields(vec!["steak_bowl_price"]));
    }
}
