#|======================================================================
JSON.LISP

1. Thin JSON utility wrapper around shasht — provides convenience functions.
2. Called by: llm/client.lisp, llm/stream.lisp, state/state.lisp
3. (2025-06-13) Created because shasht uses a stream-based API; these wrappers
   give us the string-in/string-out convenience we need everywhere.

    Author: liangzid
    Copyright © 2025, all rights reserved.
    Created: 13 June 2025
======================================================================|#
(in-package :xgamengine)

;; ---- JSON Encoding ----

(defun json-encode (data)
  "Encode DATA to a JSON string.
Uses shasht:write-json with plist-as-object mode and lowercase symbol keys.
This ensures keyword symbols like :max_tokens become \"max_tokens\" in JSON.
Returns the JSON string."
  (let ((shasht:*write-plist-as-object* t)
        (shasht:*symbol-name-function*
         (lambda (sym) (string-downcase (symbol-name sym)))))
    (with-output-to-string (s)
      (shasht:write-json data s))))

(defun json-encode-plist (plist)
  "Encode a plist as a JSON object string.
Convenience wrapper — equivalent to json-encode."
  (json-encode plist))

;; ---- JSON Decoding ----

(defun json-decode (json-string)
  "Parse a JSON string into a Lisp data structure.
Returns nested alists for objects and lists for arrays.
Uses shasht:read-json."
  (let ((shasht:*read-default-object-format* :alist)
        (shasht:*read-default-array-format* :list))
    (with-input-from-string (s json-string)
      (shasht:read-json s))))

;; ---- Conversion Helpers ----

(defun alist-to-plist (alist &key (key-transform #'identity))
  "Convert an alist (list of (key . value)) to a plist.
If KEY-TRANSFORM is provided, it is applied to each key.
By default, string keys are converted to keywords (upcased and interned)."
  (let ((result nil))
    (dolist (pair alist)
      (let ((key (car pair))
            (value (cdr pair)))
        (push (funcall key-transform key) result)
        (push value result)))
    (nreverse result)))

(defun string-keyword (str)
  "Convert a string to a keyword symbol (uppercased, matching reader behavior).
Example: (string-keyword \"id\") => :ID, which matches how the reader reads :id"
  (intern (string-upcase str) :keyword))

(defun plist-to-alist (plist)
  "Convert a plist (alternating keyword/value) to an alist with string keys.
Example: (:role \"user\" :content \"hi\") => ((\"role\" . \"user\") (\"content\" . \"hi\"))"
  (let ((result nil))
    (loop for (key value) on plist by #'cddr
          do (push (cons (string-downcase (symbol-name key)) value) result))
    (nreverse result)))

(defun plists-to-alists (plists)
  "Convert a list of plists to a list of alists with string keys."
  (mapcar #'plist-to-alist plists))

;; ---- JSON Access ----

(defun json-get (object key)
  "Get the value for KEY from a parsed JSON OBJECT.
OBJECT can be an alist, plist, list, or vector.
KEY can be a string, keyword, or integer (for list/vector indexing).
Returns the value or NIL if not found."
  (cond
    ((null object) nil)
    ((integerp key)
     ;; List or vector indexing
     (cond
       ((vectorp object)
        (when (< key (length object))
          (aref object key)))
       ((listp object)
        (nth key object))
       (t nil)))
    (t
     ;; Try both alist and plist lookup
     (or
      ;; Try as alist with string comparison
      (cdr (assoc (typecase key
                    (keyword (string-downcase (symbol-name key)))
                    (string key)
                    (t (format nil "~a" key)))
                  object
                  :test #'string=))
      ;; Try as plist
      (when (keywordp key)
        (getf object key))))))

(defun json-get-in (object &rest keys)
  "Navigate nested JSON structure with multiple KEYS.
Each key is passed to json-get sequentially.
Returns the value at the end of the path, or NIL if any key is missing.
Example: (json-get-in response :|choices| 0 :|message| :|content|)"
  (let ((current object))
    (dolist (key keys)
      (setf current (json-get current key))
      (unless current
        (return-from json-get-in nil)))
    current))
