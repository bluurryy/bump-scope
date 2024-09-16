inspect_asm::alloc_iter_u32::down:
	push rbp
	push r15
	push r14
	push r13
	push r12
	push rbx
	sub rsp, 40
	mov rbx, rdi
	test rdx, rdx
	je .LBB0_4
	mov rax, rdx
	shr rax, 61
	jne .LBB0_15
	mov r15, rsi
	lea r12, [4*rdx]
	mov rcx, qword ptr [rbx]
	mov rax, qword ptr [rcx]
	mov rsi, rax
	sub rsi, qword ptr [rcx + 8]
	cmp r12, rsi
	ja .LBB0_14
	sub rax, r12
	and rax, -4
	mov qword ptr [rcx], rax
.LBB0_0:
	mov qword ptr [rsp + 8], rax
	mov qword ptr [rsp + 16], 0
	mov qword ptr [rsp + 24], rdx
	mov qword ptr [rsp + 32], rbx
	xor r13d, r13d
	lea rbx, [rsp + 8]
	xor r14d, r14d
	jmp .LBB0_2
.LBB0_1:
	mov dword ptr [rax + 4*r14], ebp
	inc r14
	mov qword ptr [rsp + 16], r14
	add r13, 4
	cmp r12, r13
	je .LBB0_3
.LBB0_2:
	mov ebp, dword ptr [r15 + r13]
	cmp qword ptr [rsp + 24], r14
	jne .LBB0_1
	mov rdi, rbx
	call bump_scope::bump_vec::BumpVec<T,A,_,_,_>::generic_grow_cold
	mov rax, qword ptr [rsp + 8]
	mov r14, qword ptr [rsp + 16]
	jmp .LBB0_1
.LBB0_3:
	mov rsi, qword ptr [rsp + 8]
	mov rbx, qword ptr [rsp + 32]
	jmp .LBB0_5
.LBB0_4:
	mov qword ptr [rsp + 24], rdx
	mov esi, 4
	xor r14d, r14d
.LBB0_5:
	lea r15, [4*r14]
	mov rax, qword ptr [rbx]
	mov rax, qword ptr [rax]
	cmp rsi, rax
	je .LBB0_7
	mov r12, rsi
	cmp r12, rax
	je .LBB0_10
.LBB0_6:
	mov rax, r12
	jmp .LBB0_13
.LBB0_7:
	mov rax, qword ptr [rsp + 24]
	lea rax, [rsi + 4*rax]
	xor r12d, r12d
	sub rax, r15
	cmovae r12, rax
	and r12, -4
	lea rax, [r15 + rsi]
	mov rdi, r12
	mov rdx, r15
	cmp rax, r12
	jbe .LBB0_8
	call qword ptr [rip + memmove@GOTPCREL]
	jmp .LBB0_9
.LBB0_8:
	call qword ptr [rip + memcpy@GOTPCREL]
.LBB0_9:
	mov rax, qword ptr [rbx]
	mov qword ptr [rax], r12
	mov qword ptr [rsp + 24], r14
	mov rax, qword ptr [rbx]
	mov rax, qword ptr [rax]
	cmp r12, rax
	jne .LBB0_6
.LBB0_10:
	mov rcx, qword ptr [rsp + 24]
	lea rcx, [rax + 4*rcx]
	xor edx, edx
	sub rcx, r15
	cmovae rdx, rcx
	and rdx, -4
	lea rcx, [r15 + rax]
	mov rdi, rdx
	sub rdi, rax
	add rdi, r12
	mov r13, rdi
	mov rsi, r12
	cmp rcx, rdx
	jbe .LBB0_11
	mov rdx, r15
	call qword ptr [rip + memmove@GOTPCREL]
	jmp .LBB0_12
.LBB0_11:
	mov rdx, r15
	call qword ptr [rip + memcpy@GOTPCREL]
.LBB0_12:
	mov rcx, qword ptr [rbx]
	mov rax, r13
	mov qword ptr [rcx], r13
.LBB0_13:
	mov rdx, r14
	add rsp, 40
	pop rbx
	pop r12
	pop r13
	pop r14
	pop r15
	pop rbp
	ret
.LBB0_14:
	mov rdi, rbx
	mov rsi, rdx
	mov r14, rdx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_slice_in_another_chunk
	mov rdx, r14
	jmp .LBB0_0
.LBB0_15:
	call qword ptr [rip + bump_scope::private::capacity_overflow@GOTPCREL]
	mov rcx, qword ptr [rsp + 8]
	mov rdx, qword ptr [rsp + 32]
	mov rdx, qword ptr [rdx]
	cmp qword ptr [rdx], rcx
	jne .LBB0_16
	mov rsi, qword ptr [rsp + 24]
	lea rcx, [rcx + 4*rsi]
	mov qword ptr [rdx], rcx
.LBB0_16:
	mov rdi, rax
	call _Unwind_Resume@PLT
