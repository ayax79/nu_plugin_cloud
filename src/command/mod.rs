use crate::CloudPlugin;

mod clear;
mod ls;
mod open;
mod rm;
mod save;
mod stub;

pub fn commands() -> Vec<Box<dyn nu_plugin::PluginCommand<Plugin = CloudPlugin>>> {
    vec![
        Box::new(clear::Clear),
        Box::new(ls::Ls),
        Box::new(open::Open),
        Box::new(rm::Remove),
        Box::new(save::Save),
        Box::new(stub::Stub),
    ]
}

#[cfg(test)]
mod tests {
    use crate::CloudPlugin;
    use nu_command::{FromCsv, Select, ToCsv};
    use nu_plugin_test_support::PluginTest;
    use nu_protocol::{record, PipelineData, Span, Value};

    #[test]
    fn test_save_open() -> Result<(), Box<dyn std::error::Error>> {
        let plugin = CloudPlugin::default();
        let mut plugin_test = PluginTest::new("polars", plugin.into())?;
        let _ = plugin_test.add_decl(Box::new(ToCsv))?;
        let _ = plugin_test.add_decl(Box::new(FromCsv))?;
        let result = plugin_test.eval_with(
            "[[a b]; [1 2]] | cloud save memory:/foo.csv | cloud open memory:/foo.csv",
            PipelineData::Empty,
        )?;
        let value = result.into_value(Span::test_data())?;
        assert_eq!(
            value,
            Value::test_list(vec![Value::test_record(record!(
                "a" => Value::test_int(1),
                "b" => Value::test_int(2),
            ))])
        );
        Ok(())
    }

    #[test]
    fn test_save_open_raw() -> Result<(), Box<dyn std::error::Error>> {
        let plugin = CloudPlugin::default();
        let mut plugin_test = PluginTest::new("polars", plugin.into())?;
        let _ = plugin_test.add_decl(Box::new(ToCsv))?;
        let _ = plugin_test.add_decl(Box::new(FromCsv))?;
        let result = plugin_test.eval_with(
            "[[a b]; [1 2]] | cloud save memory:/foo.csv | cloud open --raw memory:/foo.csv",
            PipelineData::Empty,
        )?;
        let value = result.into_value(Span::test_data())?;
        assert_eq!(value, Value::test_string("a,b\n1,2\n"));
        Ok(())
    }

    #[test]
    fn test_save_raw_open() -> Result<(), Box<dyn std::error::Error>> {
        let plugin = CloudPlugin::default();
        let mut plugin_test = PluginTest::new("polars", plugin.into())?;
        let _ = plugin_test.add_decl(Box::new(ToCsv))?;
        let _ = plugin_test.add_decl(Box::new(FromCsv))?;
        let result = plugin_test.eval_with(
            "[[a b]; [1 2]] | to csv | cloud save --raw memory:/foo.csv | cloud open memory:/foo.csv",
            PipelineData::Empty,
        )?;
        let value = result.into_value(Span::test_data())?;
        assert_eq!(
            value,
            Value::test_list(vec![Value::test_record(record!(
                "a" => Value::test_int(1),
                "b" => Value::test_int(2),
            ))])
        );
        Ok(())
    }

    #[test]
    fn test_list() -> Result<(), Box<dyn std::error::Error>> {
        let plugin = CloudPlugin::default();
        let mut plugin_test = PluginTest::new("polars", plugin.into())?;
        let _ = plugin_test.add_decl(Box::new(ToCsv))?;
        let _ = plugin_test.add_decl(Box::new(Select))?;
        let result = plugin_test.eval_with(
            "[[a b]; [1 2]] | cloud save memory:/foo.csv | cloud ls memory:/ | select name size",
            PipelineData::Empty,
        )?;
        let value = result.into_value(Span::test_data())?;
        assert_eq!(
            value,
            Value::test_list(vec![Value::test_record(record!(
                "name" => Value::test_string("foo.csv"),
                "size" => Value::test_filesize(8),
            ))])
        );
        Ok(())
    }
}
