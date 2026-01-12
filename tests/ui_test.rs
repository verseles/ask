#[cfg(test)]
mod ui_tests {
    use requestty::{prompt_one_with, Question};
    use requestty_ui::backend::{Size, TestBackend};
    use requestty_ui::events::{KeyCode, KeyEvent, TestEvents};

    #[test]
    fn test_raw_select_with_number_key_1() {
        let mut backend = TestBackend::new(Size {
            width: 80,
            height: 24,
        });
        let events = vec![
            KeyEvent::from(KeyCode::Char('1')),
            KeyEvent::from(KeyCode::Enter),
        ];
        let mut events = TestEvents::new(events);

        let question = Question::raw_select("menu")
            .message("Choose an option:")
            .choices(vec!["Option A", "Option B", "Option C"])
            .build();

        let answer = prompt_one_with(question, &mut backend, &mut events).unwrap();
        let item = answer.as_list_item().unwrap();
        assert_eq!(item.index, 0);
    }

    #[test]
    fn test_raw_select_with_number_key_2() {
        let mut backend = TestBackend::new(Size {
            width: 80,
            height: 24,
        });
        let events = vec![
            KeyEvent::from(KeyCode::Char('2')),
            KeyEvent::from(KeyCode::Enter),
        ];
        let mut events = TestEvents::new(events);

        let question = Question::raw_select("menu")
            .message("Choose an option:")
            .choices(vec!["Option A", "Option B", "Option C"])
            .build();

        let answer = prompt_one_with(question, &mut backend, &mut events).unwrap();
        let item = answer.as_list_item().unwrap();
        assert_eq!(item.index, 1);
    }

    #[test]
    fn test_raw_select_with_arrow_down() {
        let mut backend = TestBackend::new(Size {
            width: 80,
            height: 24,
        });
        let events = vec![
            KeyEvent::from(KeyCode::Down),
            KeyEvent::from(KeyCode::Enter),
        ];
        let mut events = TestEvents::new(events);

        let question = Question::raw_select("menu")
            .message("Choose an option:")
            .choices(vec!["Option A", "Option B", "Option C"])
            .build();

        let answer = prompt_one_with(question, &mut backend, &mut events).unwrap();
        let item = answer.as_list_item().unwrap();
        assert_eq!(item.index, 1);
    }

    #[test]
    fn test_confirm_yes() {
        let mut backend = TestBackend::new(Size {
            width: 80,
            height: 24,
        });
        let events = vec![
            KeyEvent::from(KeyCode::Char('y')),
            KeyEvent::from(KeyCode::Enter),
        ];
        let mut events = TestEvents::new(events);

        let question = Question::confirm("proceed")
            .message("Continue?")
            .default(false)
            .build();

        let answer = prompt_one_with(question, &mut backend, &mut events).unwrap();
        assert_eq!(answer.as_bool(), Some(true));
    }

    #[test]
    fn test_confirm_no() {
        let mut backend = TestBackend::new(Size {
            width: 80,
            height: 24,
        });
        let events = vec![
            KeyEvent::from(KeyCode::Char('n')),
            KeyEvent::from(KeyCode::Enter),
        ];
        let mut events = TestEvents::new(events);

        let question = Question::confirm("proceed")
            .message("Continue?")
            .default(true)
            .build();

        let answer = prompt_one_with(question, &mut backend, &mut events).unwrap();
        assert_eq!(answer.as_bool(), Some(false));
    }

    #[test]
    fn test_confirm_default_with_enter() {
        let mut backend = TestBackend::new(Size {
            width: 80,
            height: 24,
        });
        let events = vec![KeyEvent::from(KeyCode::Enter)];
        let mut events = TestEvents::new(events);

        let question = Question::confirm("proceed")
            .message("Continue?")
            .default(true)
            .build();

        let answer = prompt_one_with(question, &mut backend, &mut events).unwrap();
        assert_eq!(answer.as_bool(), Some(true));
    }

    #[test]
    fn test_input_with_default() {
        let mut backend = TestBackend::new(Size {
            width: 80,
            height: 24,
        });
        let events = vec![KeyEvent::from(KeyCode::Enter)];
        let mut events = TestEvents::new(events);

        let question = Question::input("model")
            .message("Model:")
            .default("gemini-3-flash")
            .build();

        let answer = prompt_one_with(question, &mut backend, &mut events).unwrap();
        assert_eq!(answer.as_string(), Some("gemini-3-flash"));
    }
}
