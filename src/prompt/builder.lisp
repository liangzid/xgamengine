#|======================================================================
BUILDER.LISP

1. Prompt builder — assembles the final messages array for LLM API calls.
2. Called by: engine.lisp → builder.lisp → loader.lisp + state.lisp + memory/short-term.lisp
3. (2025-06-13) Phase 4: full prompt assembly with state injection and NPC role cards.

    Author: liangzid
    Copyright © 2025, all rights reserved.
    Created: 13 June 2025
======================================================================|#
(in-package :xgamengine)

;; ---- System Prompt Building ----

(defun build-system-prompt (state &key (npc "qingxu"))
  "Build the complete system prompt for the LLM.
Loads world-rules.md, injects current state narrative, appends guardrails.md,
and the specified NPC role card.
STATE is a GAME-STATE struct.
NPC is the name of the NPC template to load (without '.md' extension)."
  (let* ((world-rules-raw (load-template "world-rules"))
         (narrative (state-to-narrative state))
         (guardrails-raw (load-template "guardrails"))
         (npc-card (load-template (concatenate 'string "npc-" npc)))
         ;; Render world rules with state narrative
         (world-rules (render-template world-rules-raw
                                       (list (cons "state-narrative" narrative)))))
    ;; Concatenate all parts into a single system prompt
    (format nil "~a~%~%~a~%~%~a"
            world-rules
            guardrails-raw
            npc-card)))

;; ---- Message Array Building ----

(defun build-messages (state window user-input &key (npc-name "qingxu"))
  "Build the full messages array for a chat completion API call.
STATE is the current GAME-STATE.
WINDOW is a CONVERSATION-WINDOW with prior messages.
USER-INPUT is the player's current input string.
NPC-NAME names the NPC template to use.
Returns a list of message plists suitable for the API."
  (let ((system-prompt (build-system-prompt state :npc npc-name))
        (history (get-context-messages window)))
    ;; Build the messages array
    (append (list (list :role "system" :content system-prompt))
            history
            (list (list :role "user" :content user-input)))))

;; ---- Suggestions Generator ----

(defun generate-suggestions (state)
  "Generate a list of 3 suggested next actions based on the current state.
These are pre-written suggestions based on state analysis, not AI-generated.
The UI may use these as quick-select options.
Returns a list of 3 strings."
  (let* ((realm (game-state-realm state))
         (realm-idx (realm-index realm))
         (progress (game-state-realm-progress state))
         (qi (game-state-qi state))
         (stones (game-state-spirit-stones state))
         (has-technique-p (> (length (game-state-techniques state)) 1))
         (suggestions nil))

    ;; Suggestion 1: Always offer cultivation
    (push "闭目凝神，运转功法修炼" suggestions)

    ;; Suggestion 2: Context-dependent
    (cond
      ((< qi 30)
       (push "寻找灵气充沛之处恢复灵力" suggestions))
      ((and realm-idx (< realm-idx 3) (< progress 0.8))
       (push "前往后山试剑崖磨练剑法" suggestions))
      ((> progress 0.8)
       (push "向师尊请教突破心得" suggestions))
      (t
       (push "探索宗门周边，寻找机缘" suggestions)))

    ;; Suggestion 3: Social or economic
    (cond
      ((> stones 100)
       (push "前往坊市购买丹药" suggestions))
      ((< (length (game-state-relationships state)) 2)
       (push "在宗门内走动，结识同门" suggestions))
      (has-technique-p
       (push "研习新功法，精进招式" suggestions))
      (t
       (push "向师尊请安，聆听教诲" suggestions)))

    (nreverse suggestions)))

;; ---- State Change Extraction ----

(defun extract-state-changes (assistant-response state)
  "Extract state change instructions from the assistant's response.
Uses simple pattern matching to detect keywords indicating state changes.
Returns a plist of changes to apply, or NIL if no changes detected.

NOTE: This is a simple heuristic for MVP. A more robust approach would use
a separate LLM call with JSON output mode, but that doubles API cost."
  (let ((content (getf assistant-response :content))
        (changes nil))
    (when content
      ;; Detect realm progress increases
      (when (or (str:containsp "突破" content)
                (str:containsp "进步" content)
                (str:containsp "精进" content)
                (str:containsp "提升" content)
                (str:containsp "瓶颈松动" content))
        (push (cons :realm-progress
                    (if (str:containsp "瓶颈" content)
                        0.15
                        0.05))
              changes))
      ;; Detect breakthrough events
      (when (or (str:containsp "筑基成功" content)
                (str:containsp "金丹已成" content)
                (str:containsp "元婴凝聚" content)
                (str:containsp "化神圆满" content))
        (push (cons :realm-progress 0.25) changes))
      ;; Detect qi changes
      (when (or (str:containsp "灵力消耗" content)
                (str:containsp "灵力损耗" content)
                (str:containsp "灵力枯竭" content))
        (push (cons :qi-delta -20) changes))
      (when (or (str:containsp "灵力恢复" content)
                (str:containsp "灵力充沛" content)
                (str:containsp "灵力充盈" content))
        (push (cons :qi-delta 15) changes))
      ;; Detect spirit stone changes
      (when (str:containsp "灵石" content)
        (cond
          ((or (str:containsp "获得灵石" content)
               (str:containsp "赐予灵石" content)
               (str:containsp "奖励灵石" content))
           (push (cons :spirit-stones-delta 30) changes))
          ((or (str:containsp "花费灵石" content)
               (str:containsp "灵石不足" content)
               (str:containsp "支付灵石" content))
           (push (cons :spirit-stones-delta -20) changes)))))
    (when changes
      ;; Convert alist to plist
      (let ((result nil))
        (dolist (pair changes)
          (setf (getf result (car pair)) (cdr pair)))
        result))))
