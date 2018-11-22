(
    $list_head ((
        \y ((\x (y (x x))) (\x (y (x x)))) # Y Combinator
        \self (\x
            ($if
                ($eq x 0)
                ($list_push 0 ~)
                ($list_push x (self ($sub x 1)))
            )
        )
    ) 5)
)
