inspect_asm::alloc_iter_u32::down:
	push rbp
	push r15
	push r14
	push r12
	push rbx
	sub rsp, 32
	mov rbx, rdi
	test rdx, rdx
	je .LBB0_5
	mov rax, rdx
	shr rax, 61
	jne .LBB0_11
	mov r14, rsi
	lea r15, [4*rdx]
	mov rcx, qword ptr [rbx]
	mov rax, qword ptr [rcx]
	mov rsi, rax
	sub rsi, qword ptr [rcx + 8]
	cmp r15, rsi
	ja .LBB0_10
	sub rax, r15
	and rax, -4
	mov qword ptr [rcx], rax
.LBB0_0:
	mov qword ptr [rsp], rax
	mov qword ptr [rsp + 8], 0
	mov qword ptr [rsp + 16], rdx
	mov qword ptr [rsp + 24], rbx
	xor r12d, r12d
	mov rbx, rsp
	xor edx, edx
	jmp .LBB0_2
.LBB0_1:
	mov dword ptr [rax + 4*rdx], ebp
	inc rdx
	mov qword ptr [rsp + 8], rdx
	add r12, 4
	cmp r15, r12
	je .LBB0_3
.LBB0_2:
	mov ebp, dword ptr [r14 + r12]
	cmp qword ptr [rsp + 16], rdx
	jne .LBB0_1
	mov rdi, rbx
	call bump_scope::bump_vec::BumpVec<T,A,_,_,_>::generic_grow_amortized
	mov rax, qword ptr [rsp]
	mov rdx, qword ptr [rsp + 8]
	jmp .LBB0_1
.LBB0_3:
	mov rsi, qword ptr [rsp]
	mov rbx, qword ptr [rsp + 24]
	mov rax, qword ptr [rbx]
	cmp rsi, qword ptr [rax]
	je .LBB0_6
.LBB0_4:
	mov rax, rsi
	jmp .LBB0_9
.LBB0_5:
	mov qword ptr [rsp + 16], rdx
	mov esi, 4
	xor edx, edx
	mov rax, qword ptr [rbx]
	cmp rsi, qword ptr [rax]
	jne .LBB0_4
.LBB0_6:
	mov r14, rdx
	lea rdx, [4*rdx]
	mov rax, qword ptr [rsp + 16]
	lea rax, [rsi + 4*rax]
	xor edi, edi
	sub rax, rdx
	cmovae rdi, rax
	and rdi, -4
	lea rax, [rdx + rsi]
	mov r15, rdi
	cmp rax, rdi
	jbe .LBB0_7
	call qword ptr [rip + memmove@GOTPCREL]
	jmp .LBB0_8
.LBB0_7:
	call qword ptr [rip + memcpy@GOTPCREL]
.LBB0_8:
	mov rcx, qword ptr [rbx]
	mov rax, r15
	mov qword ptr [rcx], r15
	mov rdx, r14
.LBB0_9:
	add rsp, 32
	pop rbx
	pop r12
	pop r14
	pop r15
	pop rbp
	ret
.LBB0_10:
	mov rdi, rbx
	mov rsi, rdx
	mov r12, rdx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_slice_in_another_chunk
	mov rdx, r12
	jmp .LBB0_0
.LBB0_11:
	call qword ptr [rip + bump_scope::private::capacity_overflow@GOTPCREL]
	mov rcx, qword ptr [rsp]
	mov rdx, qword ptr [rsp + 24]
	mov rdx, qword ptr [rdx]
	cmp qword ptr [rdx], rcx
	jne .LBB0_12
	mov rsi, qword ptr [rsp + 16]
	lea rcx, [rcx + 4*rsi]
	mov qword ptr [rdx], rcx
.LBB0_12:
	mov rdi, rax
	call _Unwind_Resume@PLT
