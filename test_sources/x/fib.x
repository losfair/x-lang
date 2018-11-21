(
    (
        (\y ((\x (y (x x))) (\x (y (x x))))) # Y Combinator
        (\self (\x
            ($if
                ($or ($eq x 1) ($eq x 2))
                1
                ($add (self ($sub x 1)) (self ($sub x 2)))
            )
        ))
    ) (5)
)
