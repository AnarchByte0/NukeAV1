#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(unused)]

use std::os::raw::{c_char, c_int, c_void};
use crate::*;

pub struct UIBuilder<'a> {
    pub suite: &'a PrSDKExportParamSuite,
    pub plugin_id: csSDK_uint32,
    pub group_index: csSDK_int32,
}

impl<'a> UIBuilder<'a> {
    pub fn new(suite: &'a PrSDKExportParamSuite, plugin_id: csSDK_uint32) -> Option<Self> {
        let mut group_index = 0;
        if let Some(add_multi_group) = suite.AddMultiGroup {
            let err = unsafe { add_multi_group(plugin_id, &mut group_index) };
            if err == 0 {
                return Some(Self { suite, plugin_id, group_index });
            }
        }
        None
    }

    pub fn add_group(&self, parent_id: &[u8], group_id: &[u8], name: &str) {
        if let Some(add_param_group) = self.suite.AddParamGroup {
            unsafe {
                add_param_group(
                    self.plugin_id,
                    self.group_index,
                    parent_id.as_ptr() as *const c_char,
                    group_id.as_ptr() as *const c_char,
                    crate::leak_utf16(name),
                    0, 0, 0
                );
            }
        }
    }

    pub fn set_param_name(&self, param_id: &[u8], name: &str) {
        if let Some(set_param_name) = self.suite.SetParamName {
            unsafe {
                set_param_name(
                    self.plugin_id,
                    self.group_index,
                    param_id.as_ptr() as *const c_char,
                    crate::leak_utf16(name)
                );
            }
        }
    }

    pub fn add_dropdown(&self, parent_id: &[u8], param_id: &[u8], name: &str, default_val: i32) {
        if let Some(add_param) = self.suite.AddParam {
            unsafe {
                let mut param_info: exNewParamInfo = core::mem::zeroed();
                param_info.structVersion = 1;
                
                let id_len = param_id.len().min(255);
                for i in 0..id_len { param_info.identifier[i] = param_id[i] as c_char; }
                param_info.identifier[id_len] = 0;

                let name_ptr = core::ptr::addr_of_mut!(param_info.name) as *mut prUTF16Char;
                crate::str_to_utf16(name, name_ptr, 256);

                param_info.paramType = exParamType_exParamType_int;
                param_info.flags = 0;
                param_info.paramValues.value.__bindgen_anon_1.intValue = default_val;
                
                add_param(
                    self.plugin_id,
                    self.group_index,
                    parent_id.as_ptr() as *const c_char,
                    &param_info
                );
            }
        }
    }

    pub fn add_int_param(&self, parent_id: &[u8], param_id: &[u8], name: &str, default_val: i32, min_val: i32, max_val: i32) {
        if let Some(add_param) = self.suite.AddParam {
            unsafe {
                let mut param_info: exNewParamInfo = core::mem::zeroed();
                param_info.structVersion = 1;
                
                let id_len = param_id.len().min(255);
                for i in 0..id_len { param_info.identifier[i] = param_id[i] as c_char; }
                param_info.identifier[id_len] = 0;

                let name_ptr = core::ptr::addr_of_mut!(param_info.name) as *mut prUTF16Char;
                crate::str_to_utf16(name, name_ptr, 256);

                param_info.paramType = exParamType_exParamType_int;
                param_info.flags = 0;
                
                param_info.paramValues.rangeMin.__bindgen_anon_1.intValue = min_val;
                param_info.paramValues.rangeMax.__bindgen_anon_1.intValue = max_val;
                param_info.paramValues.value.__bindgen_anon_1.intValue = default_val;
                
                add_param(
                    self.plugin_id,
                    self.group_index,
                    parent_id.as_ptr() as *const c_char,
                    &param_info
                );
            }
        }
    }

    pub fn add_time_param(&self, parent_id: &[u8], param_id: &[u8], name: &str, default_val: i64, min_val: i64, max_val: i64) {
        if let Some(add_param) = self.suite.AddParam {
            unsafe {
                let mut param_info: exNewParamInfo = core::mem::zeroed();
                param_info.structVersion = 1;
                
                let id_len = param_id.len().min(255);
                for i in 0..id_len { param_info.identifier[i] = param_id[i] as c_char; }
                param_info.identifier[id_len] = 0;

                let name_ptr = core::ptr::addr_of_mut!(param_info.name) as *mut prUTF16Char;
                crate::str_to_utf16(name, name_ptr, 256);

                param_info.paramType = exParamType_exParamType_ticksFrameRate;
                param_info.flags = 0;
                
                param_info.paramValues.rangeMin.__bindgen_anon_1.timeValue = min_val;
                param_info.paramValues.rangeMax.__bindgen_anon_1.timeValue = max_val;
                param_info.paramValues.value.__bindgen_anon_1.timeValue = default_val;
                
                add_param(
                    self.plugin_id,
                    self.group_index,
                    parent_id.as_ptr() as *const c_char,
                    &param_info
                );
            }
        }
    }

    pub fn add_ratio_param(&self, parent_id: &[u8], param_id: &[u8], name: &str, num: i32, den: i32) {
        if let Some(add_param) = self.suite.AddParam {
            unsafe {
                let mut param_info: exNewParamInfo = core::mem::zeroed();
                param_info.structVersion = 1;
                
                let id_len = param_id.len().min(255);
                for i in 0..id_len { param_info.identifier[i] = param_id[i] as c_char; }
                param_info.identifier[id_len] = 0;

                let name_ptr = core::ptr::addr_of_mut!(param_info.name) as *mut prUTF16Char;
                crate::str_to_utf16(name, name_ptr, 256);

                param_info.paramType = exParamType_exParamType_ratio;
                param_info.flags = 0;
                
                param_info.paramValues.rangeMin.__bindgen_anon_1.ratioValue.numerator = 1;
                param_info.paramValues.rangeMin.__bindgen_anon_1.ratioValue.denominator = 100;
                param_info.paramValues.rangeMax.__bindgen_anon_1.ratioValue.numerator = 100;
                param_info.paramValues.rangeMax.__bindgen_anon_1.ratioValue.denominator = 1;
                param_info.paramValues.value.__bindgen_anon_1.ratioValue.numerator = num;
                param_info.paramValues.value.__bindgen_anon_1.ratioValue.denominator = den;
                
                add_param(
                    self.plugin_id,
                    self.group_index,
                    parent_id.as_ptr() as *const c_char,
                    &param_info
                );
            }
        }
    }

    pub fn add_float_param(&self, parent_id: &[u8], param_id: &[u8], name: &str, default_val: f64, min_val: f64, max_val: f64) {
        if let Some(add_param) = self.suite.AddParam {
            unsafe {
                let mut param_info: exNewParamInfo = core::mem::zeroed();
                param_info.structVersion = 1;
                
                let id_len = param_id.len().min(255);
                for i in 0..id_len { param_info.identifier[i] = param_id[i] as c_char; }
                param_info.identifier[id_len] = 0;

                let name_ptr = core::ptr::addr_of_mut!(param_info.name) as *mut prUTF16Char;
                crate::str_to_utf16(name, name_ptr, 256);

                param_info.paramType = exParamType_exParamType_float;
                param_info.flags = 0;
                
                param_info.paramValues.rangeMin.__bindgen_anon_1.floatValue = min_val;
                param_info.paramValues.rangeMax.__bindgen_anon_1.floatValue = max_val;
                param_info.paramValues.value.__bindgen_anon_1.floatValue = default_val;
                
                add_param(
                    self.plugin_id,
                    self.group_index,
                    parent_id.as_ptr() as *const c_char,
                    &param_info
                );
            }
        }
    }

    pub fn add_dropdown_item(&self, param_id: &[u8], value: i32, name: &str) {
        if let Some(add_constrained_pair) = self.suite.AddConstrainedValuePair {
            unsafe {
                let mut val_rec: exOneParamValueRec = core::mem::zeroed();
                val_rec.__bindgen_anon_1.intValue = value;

                add_constrained_pair(
                    self.plugin_id,
                    self.group_index,
                    param_id.as_ptr() as *const c_char,
                    &val_rec,
                    crate::leak_utf16(name)
                );
            }
        }
    }

    pub fn add_dropdown_item_time(&self, param_id: &[u8], value: i64, name: &str) {
        if let Some(add_constrained_pair) = self.suite.AddConstrainedValuePair {
            unsafe {
                let mut val_rec: exOneParamValueRec = core::mem::zeroed();
                val_rec.__bindgen_anon_1.timeValue = value;

                add_constrained_pair(
                    self.plugin_id,
                    self.group_index,
                    param_id.as_ptr() as *const c_char,
                    &val_rec,
                    crate::leak_utf16(name)
                );
            }
        }
    }

    pub fn add_dropdown_item_ratio(&self, param_id: &[u8], num: i32, den: i32, name: &str) {
        if let Some(add_constrained_pair) = self.suite.AddConstrainedValuePair {
            unsafe {
                let mut val_rec: exOneParamValueRec = core::mem::zeroed();
                val_rec.__bindgen_anon_1.ratioValue.numerator = num;
                val_rec.__bindgen_anon_1.ratioValue.denominator = den;

                add_constrained_pair(
                    self.plugin_id,
                    self.group_index,
                    param_id.as_ptr() as *const c_char,
                    &val_rec,
                    crate::leak_utf16(name)
                );
            }
        }
    }

    pub fn add_float_dropdown(&self, parent_id: &[u8], param_id: &[u8], name: &str, default_val: f64) {
        if let Some(add_param) = self.suite.AddParam {
            unsafe {
                let mut param_info: exNewParamInfo = core::mem::zeroed();
                param_info.structVersion = 1;
                
                let id_len = param_id.len().min(255);
                for i in 0..id_len { param_info.identifier[i] = param_id[i] as c_char; }
                param_info.identifier[id_len] = 0;

                let name_ptr = core::ptr::addr_of_mut!(param_info.name) as *mut prUTF16Char;
                crate::str_to_utf16(name, name_ptr, 256);

                param_info.paramType = exParamType_exParamType_float;
                param_info.flags = 0;
                
                param_info.paramValues.value.__bindgen_anon_1.floatValue = default_val;
                
                add_param(
                    self.plugin_id,
                    self.group_index,
                    parent_id.as_ptr() as *const c_char,
                    &param_info
                );
            }
        }
    }

    pub fn add_time_dropdown(&self, parent_id: &[u8], param_id: &[u8], name: &str, default_val: i64) {
        if let Some(add_param) = self.suite.AddParam {
            unsafe {
                let mut param_info: exNewParamInfo = core::mem::zeroed();
                param_info.structVersion = 1;
                
                let id_len = param_id.len().min(255);
                for i in 0..id_len { param_info.identifier[i] = param_id[i] as c_char; }
                param_info.identifier[id_len] = 0;

                let name_ptr = core::ptr::addr_of_mut!(param_info.name) as *mut prUTF16Char;
                crate::str_to_utf16(name, name_ptr, 256);

                param_info.paramType = exParamType_exParamType_ticksFrameRate;
                param_info.flags = 0;
                
                param_info.paramValues.value.__bindgen_anon_1.timeValue = default_val;
                
                add_param(
                    self.plugin_id,
                    self.group_index,
                    parent_id.as_ptr() as *const c_char,
                    &param_info
                );
            }
        }
    }

    pub fn add_dropdown_item_float(&self, param_id: &[u8], value: f64, name: &str) {
        if let Some(add_constrained_pair) = self.suite.AddConstrainedValuePair {
            unsafe {
                let mut val_rec: exOneParamValueRec = core::mem::zeroed();
                val_rec.__bindgen_anon_1.floatValue = value;

                add_constrained_pair(
                    self.plugin_id,
                    self.group_index,
                    param_id.as_ptr() as *const c_char,
                    &val_rec,
                    crate::leak_utf16(name)
                );
            }
        }
    }

    pub fn add_bool_param(&self, parent_id: &[u8], param_id: &[u8], name: &str, default_val: bool) {
        if let Some(add_param) = self.suite.AddParam {
            unsafe {
                let mut param_info: exNewParamInfo = core::mem::zeroed();
                param_info.structVersion = 1;
                
                let id_len = param_id.len().min(255);
                for i in 0..id_len { param_info.identifier[i] = param_id[i] as c_char; }
                param_info.identifier[id_len] = 0;

                let name_ptr = core::ptr::addr_of_mut!(param_info.name) as *mut prUTF16Char;
                crate::str_to_utf16(name, name_ptr, 256);

                param_info.paramType = exParamType_exParamType_bool;
                param_info.flags = 0;
                param_info.paramValues.value.__bindgen_anon_1.intValue = if default_val { 1 } else { 0 };
                
                add_param(
                    self.plugin_id,
                    self.group_index,
                    parent_id.as_ptr() as *const c_char,
                    &param_info
                );
            }
        }
    }

    pub fn set_params_version(&self, version: csSDK_int32) {
        if let Some(set_params_version) = self.suite.SetParamsVersion {
            unsafe {
                set_params_version(self.plugin_id, version);
            }
        }
    }
}
