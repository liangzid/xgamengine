#|======================================================================
SHORT-TERM.LISP

1. Short-term memory — conversation window with truncation logic.
2. Called by: engine.lisp → memory/short-term.lisp → maintains rolling message history.
3. (2025-06-13) Phase 3: ring-buffer-like conversation window with smart truncation.

    Author: liangzid
    Copyright © 2025, all rights reserved.
    Created: 13 June 2025
======================================================================|#
(in-package :xgamengine)

;; ---- Configuration ----

(defparameter *max-history-rounds* 12
  "Maximum number of user-assistant message pairs to retain in the conversation window.")

;; ---- Conversation Window ----

(defstruct conversation-window
  "Stores recent conversation turns for context injection into LLM requests.
Each entry is a plist with keys :role, :content (and optionally :reasoning for thinking mode)."
  (messages nil :type list)   ; list of message plists, oldest first
  (round-count 0 :type integer))

;; The defstruct auto-generates make-conversation-window with default values.
;; No explicit constructor needed.  

;; ---- Adding Turns ----

(defun append-turn! (window user-msg assistant-msg)
  "Append a user-assistant message pair to the conversation WINDOW.
USER-MSG and ASSISTANT-MSG are plists with keys :role and :content.
Mutates WINDOW and returns it.
If the window exceeds *max-history-rounds*, oldest turns are trimmed."
  (push (list :role "user" :content user-msg)
        (conversation-window-messages window))
  (push assistant-msg
        (conversation-window-messages window))
  (incf (conversation-window-round-count window))
  ;; Reverse to maintain chronological order (oldest first)
  ;; Then trim if needed
  (setf (conversation-window-messages window)
        (nreverse (conversation-window-messages window)))
  (truncate-if-needed window)
  window)

(defun truncate-if-needed (window)
  "Trim the conversation window if it exceeds the maximum number of rounds.
Keeps the first 2 messages (usually system prompt + first turn) and the last
N turns, where N = *max-history-rounds*. Mutates WINDOW."
  (let ((msgs (conversation-window-messages window))
        (max-msgs (* 2 *max-history-rounds*)))  ; each round = user + assistant = 2 msgs
    (when (> (length msgs) max-msgs)
      ;; Keep the first 2 messages (system-level context) + most recent turns
      (let ((head (subseq msgs 0 4))   ; first 2 turns (4 messages)
            (tail (subseq msgs (- (length msgs) (- max-msgs 4)))))
        (setf (conversation-window-messages window)
              (append head tail)))))
  window)

;; ---- Retrieving Context Messages ----

(defun get-context-messages (window)
  "Return the list of conversation messages suitable for sending to the LLM API.
The messages are already in chronological order (oldest first)."
  (conversation-window-messages window))

(defun get-recent-assistant-messages (window &optional (n 3))
  "Return the last N assistant messages from the window.
Useful for generating suggestions or summarizing recent narrative."
  (let ((result nil))
    (dolist (msg (reverse (conversation-window-messages window)))
      (when (string= (getf msg :role) "assistant")
        (push (getf msg :content) result)
        (when (>= (length result) n)
          (return))))
    result))
